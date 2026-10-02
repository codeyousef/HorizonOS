"""Rootless host VM lifecycle. No SSH execution, mounts or process-name kills."""
import json
import os
from pathlib import Path
import shutil
import socket
import stat
import struct
import time

from .config import invalid, project_path, read_json
from .doctor import qemu_capabilities, tcp_probe
from .errors import DevctlError, ExitCode
from .provision import DISK_SERIAL, digest_file, failure, operation_lock, private_directory, run, validate_plan, write_json_new


def load_record(config):
    path = project_path(config.root, ".local/provisioning.json", ".local")
    if not path.exists():
        raise failure(ExitCode.UNMET_PREREQUISITE, "NOT_PROVISIONED", "Run vm create before starting the bootstrap VM")
    record = read_json(path)
    if not isinstance(record, dict) or record.get("schema_version") != 1 or record.get("state") != "prepared":
        raise invalid("Unsupported provisioning record")
    validate_plan(config, record.get("plan"))
    if record.get("authorized_uuid") != record["plan"]["guest_uuid"] or record.get("authorized_operation") != "provision-fresh-virtual-disk":
        raise failure(ExitCode.AUTHORIZATION_NEEDED, "AUTHORIZATION_NEEDED", "Missing bootstrap authorization")
    info = config.paths["disk_image"].stat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_dev != record.get("disk_device") or info.st_ino != record.get("disk_inode"):
        raise failure(ExitCode.TARGET_MISMATCH, "TARGET_MISMATCH", "Virtual disk identity changed")
    for field, checksum in (("seed_iso", "seed_sha256"), ("firmware_code", "firmware_sha256")):
        candidate = Path(record[field])
        if candidate.is_symlink() or not candidate.is_file() or digest_file(candidate) != record[checksum]:
            raise failure(ExitCode.VERIFICATION_FAILURE, "BOOTSTRAP_DIGEST_MISMATCH", f"{field} failed verification")
    for field in ("seed_iso",):
        relative = Path(record[field]).relative_to(config.root)
        if project_path(config.root, str(relative), ".local/vm") != Path(record[field]):
            raise invalid("Bootstrap media escapes the VM directory")
    media = record["media"]
    candidate = Path(media["path"])
    relative = candidate.relative_to(config.root)
    if project_path(config.root, str(relative), ".local/vm") != candidate or candidate.is_symlink() or not candidate.is_file() or digest_file(candidate) != media["sha256"]:
        raise failure(ExitCode.VERIFICATION_FAILURE, "MEDIA_DIGEST_MISMATCH", "Installer media failed verification")
    return record


def lifecycle(config, action, *, display="gtk", bootstrap=False):
    if action == "start" and not bootstrap:
        raise failure(ExitCode.UNSUPPORTED_CAPABILITY, "UNSUPPORTED_CAPABILITY", "Normal guest startup requires enrollment; use --bootstrap for the prepared installer")
    load_record(config)
    with operation_lock(config.root):
        if action == "start":
            return start(config, display, bootstrap)
        if action == "console":
            return console(config)
        if action == "stop":
            return stop(config)
        raise invalid("Unknown lifecycle operation")


def qemu_arguments(config, record, display):
    if display not in ("gtk", "none"):
        raise invalid("Only local GTK or headless display is supported")
    if config.values["ssh_host"] != "127.0.0.1":
        raise invalid("The initial QEMU provider supports IPv4 loopback forwarding only")
    paths = [*config.paths.values(), Path(record["seed_iso"]), Path(record["firmware_code"]), Path(record["media"]["path"])]
    if any("," in str(path) or "\n" in str(path) for path in paths):
        raise invalid("QEMU option paths cannot contain commas or newlines")
    if any(len(os.fsencode(config.paths[field])) >= 104 for field in ("qmp_socket", "serial_socket")):
        raise invalid("VM socket path exceeds portable Unix socket limit")
    executable = shutil.which("qemu-system-x86_64")
    if not executable:
        raise failure(ExitCode.UNMET_PREREQUISITE, "MISSING_TOOL", "QEMU x86-64 is required")
    return [executable, "-name", config.values["name"], "-machine", "q35,accel=kvm", "-cpu", "host",
            "-smp", str(config.values["vcpus"]), "-m", str(config.values["memory_mib"]), "-uuid", record["plan"]["guest_uuid"],
            "-drive", f"if=pflash,format=raw,readonly=on,file={record['firmware_code']}",
            "-drive", f"if=pflash,format=raw,file={config.paths['nvram_file']}",
            "-drive", f"if=none,id=rootdisk,format=qcow2,file={config.paths['disk_image']}",
            "-device", f"virtio-blk-pci,drive=rootdisk,serial={DISK_SERIAL}",
            "-drive", f"file={record['media']['path']},media=cdrom,readonly=on", "-drive", f"file={record['seed_iso']},media=cdrom,readonly=on",
            "-netdev", f"user,id=net0,hostfwd=tcp:127.0.0.1:{config.values['ssh_port']}-:22", "-device", "virtio-net-pci,netdev=net0",
            "-device", "virtio-vga", "-display", display,
            "-qmp", f"unix:{config.paths['qmp_socket']},server=on,wait=off",
            "-chardev", f"socket,id=serial0,path={config.paths['serial_socket']},server=on,wait=off,logfile={config.paths['serial_log']}",
            "-serial", "chardev:serial0", "-boot", "order=d,menu=on", "-daemonize", "-pidfile", str(config.root / ".local/vm/qemu.pid")]


def process_identity(pid, proc=Path("/proc")):
    entry = proc / str(pid)
    try:
        info = entry.stat()
        fields = (entry / "stat").read_text().rsplit(")", 1)[1].split()
        args = (entry / "cmdline").read_bytes().rstrip(b"\0").split(b"\0")
        return {"pid": pid, "uid": info.st_uid, "start_ticks": int(fields[19]), "executable": str((entry / "exe").resolve(strict=True)), "arguments": [os.fsdecode(a) for a in args]}
    except (OSError, ValueError, IndexError) as error:
        raise failure(ExitCode.TARGET_MISMATCH, "PROCESS_IDENTITY_MISMATCH", "The recorded QEMU process no longer exists or cannot be identified") from error


def verify_process(expected):
    observed = process_identity(expected["pid"])
    if observed != expected or observed["uid"] != os.getuid():
        raise failure(ExitCode.TARGET_MISMATCH, "PROCESS_IDENTITY_MISMATCH", "PID, start time, executable, user or exact QEMU arguments changed")


class QMP:
    def __init__(self, config, process, guest_uuid):
        verify_process(process)
        self.process = process
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.settimeout(3)
        self.socket.connect(str(config.paths["qmp_socket"]))
        pid, uid, _ = struct.unpack("3i", self.socket.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i")))
        if pid != process["pid"] or uid != os.getuid():
            self.socket.close()
            raise failure(ExitCode.TARGET_MISMATCH, "QMP_PEER_MISMATCH", "QMP peer is not the recorded QEMU process")
        self.reader = self.socket.makefile("rb")
        greeting = self.read()
        if "QMP" not in greeting:
            self.close()
            raise failure(ExitCode.VERIFICATION_FAILURE, "QMP_PROTOCOL_FAILED", "Missing QMP greeting")
        self.command("qmp_capabilities")
        if self.command("query-uuid").get("UUID", "").lower() != guest_uuid:
            self.close()
            raise failure(ExitCode.TARGET_MISMATCH, "VM_UUID_MISMATCH", "QMP UUID differs from the provisioned VM")

    def read(self):
        line = self.reader.readline(65537)
        if len(line) > 65536 or not line.endswith(b"\n"):
            raise failure(ExitCode.VERIFICATION_FAILURE, "QMP_PROTOCOL_FAILED", "Invalid or oversized QMP response")
        return json.loads(line)

    def command(self, name):
        if name not in ("qmp_capabilities", "query-uuid", "query-block", "query-status", "quit"):
            raise invalid("Unregistered QMP lifecycle operation")
        verify_process(self.process)
        self.socket.sendall((json.dumps({"execute": name, "id": name}) + "\n").encode())
        for _ in range(64):
            response = self.read()
            if response.get("id") == name:
                if "error" in response:
                    raise failure(ExitCode.OPERATION_FAILURE, "QMP_OPERATION_FAILED", f"QMP rejected {name}")
                return response["return"]
        raise failure(ExitCode.TIMEOUT, "QMP_TIMEOUT", "No matching lifecycle response")

    def close(self):
        self.reader.close()
        self.socket.close()


def verify_block(client, config):
    blocks = client.command("query-block")
    matches = [b for b in blocks if b.get("device") == "rootdisk"]
    if len(matches) != 1 or matches[0].get("inserted", {}).get("file") != str(config.paths["disk_image"]):
        raise failure(ExitCode.TARGET_MISMATCH, "VM_DISK_MISMATCH", "QMP root disk does not match the configured virtual disk")


def start(config, display, bootstrap):
    if not bootstrap:
        raise failure(ExitCode.UNSUPPORTED_CAPABILITY, "UNSUPPORTED_CAPABILITY", "Normal guest startup requires enrollment; use --bootstrap only for the prepared installer")
    record = load_record(config)
    private_directory(config.root, ".local/vm")
    capabilities = qemu_capabilities()
    if not capabilities["probe_complete"] or not capabilities["virtio_vga"] or display == "gtk" and not capabilities["gtk"]:
        raise failure(ExitCode.UNMET_PREREQUISITE, "QEMU_DEVICE_UNAVAILABLE", "QEMU must provide virtio-vga and the requested display backend; doctor --host reports missing modules")
    state = config.root / ".local/vm/process.json"
    pidfile = config.root / ".local/vm/qemu.pid"
    if state.exists() or pidfile.exists() or any(config.paths[key].exists() for key in ("qmp_socket", "serial_socket")):
        raise failure(ExitCode.TARGET_MISMATCH, "EXISTING_VM_CONTROL", "Refusing to attach to or overwrite existing VM control artifacts")
    if tcp_probe(config.values["ssh_host"], config.values["ssh_port"])["reachable"] is not False:
        raise failure(ExitCode.UNMET_PREREQUISITE, "PORT_UNAVAILABLE", "Cannot prove the configured loopback port is free")
    args = qemu_arguments(config, record, display)
    run(args, timeout=15)
    pid = int(pidfile.read_text().strip())
    process = process_identity(pid)
    if process["arguments"] != args or process["executable"] != str(Path(args[0]).resolve()) or process["uid"] != os.getuid():
        raise failure(ExitCode.TARGET_MISMATCH, "PROCESS_IDENTITY_MISMATCH", "Started process does not match the exact QEMU launch")
    # Retain process identity before contacting QMP, including on later failure.
    write_json_new(state, process)
    client = QMP(config, process, record["plan"]["guest_uuid"])
    try:
        verify_block(client, config)
        status = client.command("query-status")
    finally:
        client.close()
    return ExitCode.SUCCESS, {"state": "bootstrap-running", "guest_identity_verified": False, "qmp_uuid_verified": True, "pid": pid, "qemu_status": status, "display": display, "message": "Use the local GTK installer console; no SSH guest operation is enabled."}


def console(config):
    record = load_record(config)
    process = read_json(project_path(config.root, ".local/vm/process.json", ".local/vm"))
    client = QMP(config, process, record["plan"]["guest_uuid"])
    try:
        verify_block(client, config)
    finally:
        client.close()
    return ExitCode.SUCCESS, {"guest_uuid": record["plan"]["guest_uuid"], "message": "Use the local QEMU GTK display. Mount AIOS_SEED read-only at /run/aios-seed, then run sudo bash /run/aios-seed/bootstrap.sh in the NixOS installer console.", "guest_identity_verified": False}


def stop(config):
    # Host power control is tied to the exact QMP process/UUID/disk, and works
    # without SSH. It never executes a guest command or sends a process signal.
    record = load_record(config)
    state = project_path(config.root, ".local/vm/process.json", ".local/vm")
    process = read_json(state)
    client = QMP(config, process, record["plan"]["guest_uuid"])
    try:
        verify_block(client, config)
        client.command("quit")
    finally:
        client.close()
    for _ in range(30):
        try:
            process_identity(process["pid"])
        except DevctlError:
            # Only disappearance of this exact PID allows stale-artifact cleanup.
            if not (Path("/proc") / str(process["pid"])).exists():
                for path in (state, config.root / ".local/vm/qemu.pid", config.paths["qmp_socket"], config.paths["serial_socket"]):
                    if path.is_symlink():
                        raise invalid("Control artifacts must not become symlinks")
                    path.unlink(missing_ok=True)
                return ExitCode.SUCCESS, {"state": "stopped", "guest_uuid": record["plan"]["guest_uuid"], "guest_identity_verified": False}
            raise
        time.sleep(0.1)
    raise failure(ExitCode.TIMEOUT, "VM_STOP_TIMEOUT", "Recorded QEMU process did not exit; no signal escalation was attempted")
