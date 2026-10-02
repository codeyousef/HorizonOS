"""Rootless host VM lifecycle. No SSH execution, mounts or process-name kills."""
import json
import base64
import hashlib
import os
from pathlib import Path
import re
import shutil
import socket
import stat
import struct
import time

from .config import invalid, project_path, read_json
from .doctor import qemu_capabilities, tcp_probe
from .errors import DevctlError, ExitCode
from .provision import DISK_SERIAL, digest_file, failure, operation_lock, private_directory, run, validate_plan, write_json_new


# This is a single registered bootstrap operation, not a shell/keyboard RPC.
# The verified official ISO supplies the console; the seed script checks guest
# OS/DMI/virtual disk/freshness/authorization before its first disk mutation.
BOOTSTRAP_CONSOLE = (
    "sudo bash -c 'exec > /dev/ttyS0 2>&1; "
    "mkdir -p /run/aios-seed && "
    "mount -o ro /dev/disk/by-label/AIOS_SEED /run/aios-seed && "
    "bash /run/aios-seed/bootstrap.sh; "
    "status=$?; printf \"AIOS_BOOTSTRAP_EXIT=%s\\n\" \"$status\"'"
)
BOOTSTRAP_INSPECT = (
    "sudo bash -c 'exec > /dev/ttyS0 2>&1; "
    "cat /etc/os-release; systemd-detect-virt --vm; cat /sys/class/dmi/id/product_uuid; "
    "lsblk -o NAME,SERIAL,FSTYPE,UUID; findmnt /mnt; ls /mnt/etc/nixos'"
)


def encoded_console_script(script, sentinel):
    encoded = base64.b64encode(script.encode())
    if len(encoded) > 65536:
        raise invalid("Registered console script exceeds 64 KiB")
    command = ("sudo bash -c 'exec > /dev/ttyS0 2>&1; stty -F /dev/ttyS0 raw -echo; "
               "printf \"AIOS_CONSOLE_READY\\n\"; head -c " + str(len(encoded)) +
               " < /dev/ttyS0 | base64 -d > /run/aios-console-action.sh; bash /run/aios-console-action.sh; "
               "status=$?; printf \"" + sentinel + "=%s\\n\" \"$status\"'")
    return command, encoded


def finish_console_command(record):
    # Only canonical UUIDs from the already validated provisioning record enter
    # this registered script. The generated tester secret never leaves the guest.
    plan = record["plan"]
    script = """#!/usr/bin/env bash
set -euo pipefail
test "$(sed -n 's/^ID=//p' /etc/os-release)" = nixos
test "$(systemd-detect-virt --vm)" = kvm
test "$(cat /sys/class/dmi/id/product_uuid)" = GUEST_UUID
test "$(lsblk --nodeps --noheadings --output SERIAL /dev/vda | xargs)" = AIOS_DEV_ROOT
test "$(findmnt --noheadings --output UUID /mnt)" = INSTALLATION_UUID
nixos-enter --root /mnt -c 'set -eu
test "$(cat /etc/aios/installation-uuid)" = INSTALLATION_UUID
test "$(cat /etc/aios/guest-role)" = development
umask 077
head -c 48 /dev/urandom | base64 > /root/.aios-tester-secret
{ printf "tester:"; cat /root/.aios-tester-secret; } | chpasswd
'
umount -R /mnt
sync
""".replace("GUEST_UUID", plan["guest_uuid"]).replace("INSTALLATION_UUID", plan["installation_uuid"])
    return encoded_console_script(script, "AIOS_FINISH_EXIT")


def audit_console_command(config, record):
    from .guest import load_trust
    fingerprint = load_trust(config)["host_key_fingerprint"]
    if not re.fullmatch(r"SHA256:[A-Za-z0-9/+]{43}", fingerprint):
        raise invalid("Invalid pinned SSH fingerprint")
    plan = record["plan"]
    source = Path(__file__).resolve().parents[1] / "guest/layout_audit.py"
    encoded = base64.b64encode(source.read_bytes()).decode()
    script = """#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
test "$EUID" = 0
test "$(sed -n 's/^ID=//p' /etc/os-release | tr -d '\"')" = nixos
test "$(systemd-detect-virt --vm)" = kvm
test "$(tr '[:upper:]' '[:lower:]' </sys/class/dmi/id/product_uuid)" = GUEST_UUID
test "$(cat /sys/class/block/vda/serial)" = AIOS_DEV_ROOT
test "$(readlink -f /sys/class/block/vda/device/driver)" = /sys/bus/virtio/drivers/virtio_blk
test "$(lsblk --nodeps --noheadings --output NAME,SERIAL,TYPE | awk '$2 == \"AIOS_DEV_ROOT\" && $3 == \"disk\" {print $1}' | xargs)" = vda
test "$(lsblk --list --noheadings --output NAME /dev/vda | wc -l)" = 3
printf 'AIOS_LAYOUT_INSTALLER_KERNEL=%s\n' "$(uname -r)"
sgdisk --verify /dev/vda
test "$(sgdisk --print /dev/vda | sed -n 's/^Disk identifier (GUID): //p' | tr '[:upper:]' '[:lower:]')" = GUEST_UUID
test "$(lsblk --bytes --noheadings --output SIZE /dev/vda1 | xargs)" = 1073741824
test "$(lsblk --noheadings --output PARTLABEL /dev/vda1 | xargs)" = AIOS_DEV_EFI
test "$(lsblk --noheadings --output PARTLABEL /dev/vda2 | xargs)" = AIOS_DEV_ROOT
test "$(lsblk --noheadings --output PARTTYPE /dev/vda1 | xargs)" = c12a7328-f81f-11d2-ba4b-00a0c93ec93b
test "$(lsblk --noheadings --output PARTTYPE /dev/vda2 | xargs)" = 0fc63daf-8483-4772-8e79-3d69d8477de4
test "$(lsblk --noheadings --output FSTYPE /dev/vda1 | xargs)" = vfat
test "$(lsblk --noheadings --output FSTYPE /dev/vda2 | xargs)" = btrfs
test "$(lsblk --noheadings --output UUID /dev/vda2 | xargs)" = INSTALLATION_UUID
! mountpoint -q /mnt
mount -o ro,rescue=nologreplay,subvol=@root /dev/vda2 /mnt
trap 'umount -R /mnt' EXIT
options=$(findmnt --noheadings --output OPTIONS /mnt)
printf '%s\n' \"$options\" | tr ',' '\n' | grep -Fx -- ro
printf '%s\n' \"$options\" | tr ',' '\n' | grep -Fx -- rescue=nologreplay
subvolumes=$(btrfs subvolume list /mnt | awk '{print $NF}')
for name in @root @home @nix @var; do printf '%s\n' "$subvolumes" | grep -Fx -- "$name"; done
for name in var nix home; do
  mount -o ro,rescue=nologreplay,subvol=@$name /dev/vda2 /mnt/$name
  test "$(findmnt --noheadings --output UUID /mnt/$name)" = INSTALLATION_UUID
done
mount -o ro,umask=0077 /dev/vda1 /mnt/boot
mount --bind /dev /mnt/dev
mount -o remount,bind,ro /mnt/dev
chroot /mnt /nix/var/nix/profiles/system/sw/bin/bash -c 'set -eu
export PATH=/nix/var/nix/profiles/system/sw/bin
test "$(cat /etc/aios/installation-uuid)" = INSTALLATION_UUID
test "$(cat /etc/aios/guest-role)" = development
test "$(cat /etc/aios/expected-dmi-uuid)" = GUEST_UUID
'
test "$(ssh-keygen -lf /mnt/etc/ssh/ssh_host_ed25519_key.pub -E sha256 | awk '{print $2}')" = HOST_FINGERPRINT
printf 'AIOS_LAYOUT_STORAGE GPT=verified EFI_BYTES=1073741824 ROOT_FS=btrfs UUID=%s DMI=%s RO=true NOLOGREPLAY=true\n' INSTALLATION_UUID GUEST_UUID
chroot /mnt /nix/var/nix/profiles/system/sw/bin/python3 -c "$(printf %s AUDIT_SOURCE | base64 -d)" GUEST_UUID INSTALLATION_UUID HOST_FINGERPRINT
journalctl --directory=/mnt/var/log/journal -u sshd.service --no-pager -n 30 || true
""".replace("GUEST_UUID", plan["guest_uuid"]).replace("INSTALLATION_UUID", plan["installation_uuid"]).replace("HOST_FINGERPRINT", fingerprint).replace("AUDIT_SOURCE", encoded)
    return encoded_console_script(script, "AIOS_AUDIT_EXIT")


def repair_console_command(config, record):
    from .guest import load_trust
    from .provision import source_files
    trust = load_trust(config)
    fingerprint = trust["host_key_fingerprint"]
    if not re.fullmatch(r"SHA256:[A-Za-z0-9/+]{43}", fingerprint):
        raise invalid("Invalid pinned SSH fingerprint")
    relative = Path("nix/machines/aios-dev/bootstrap.nix")
    identity_relative = Path("tools/guest/identity.py")
    approved = source_files(config.root)
    if relative not in approved or identity_relative not in approved:
        raise invalid("Bootstrap repair source must be reviewed tracked source")
    source = base64.b64encode((config.root / relative).read_bytes()).decode()
    identity_source = base64.b64encode((config.root / identity_relative).read_bytes()).decode()
    script = """#!/usr/bin/env bash
set -euo pipefail
test "$(sed -n 's/^ID=//p' /etc/os-release)" = nixos
test "$(systemd-detect-virt --vm)" = kvm
test "$(cat /sys/class/dmi/id/product_uuid)" = GUEST_UUID
test "$(lsblk --nodeps --noheadings --output SERIAL /dev/vda | xargs)" = AIOS_DEV_ROOT
test "$(readlink -f /sys/class/block/vda/device/driver)" = /sys/bus/virtio/drivers/virtio_blk
test "$(lsblk --noheadings --output UUID /dev/vda2 | xargs)" = INSTALLATION_UUID
mount -o ro,rescue=nologreplay,subvol=@root /dev/vda2 /mnt
trap 'umount -R /mnt' EXIT
mount -o ro,rescue=nologreplay,subvol=@nix /dev/vda2 /mnt/nix
chroot /mnt /nix/var/nix/profiles/system/sw/bin/bash -c 'set -eu
export PATH=/nix/var/nix/profiles/system/sw/bin
test "$(cat /etc/aios/installation-uuid)" = INSTALLATION_UUID
test "$(cat /etc/aios/guest-role)" = development
test "$(sed -n "s/^ID=//p" /etc/os-release)" = nixos
'
test "$(ssh-keygen -lf /mnt/etc/ssh/ssh_host_ed25519_key.pub | awk '{print $2}')" = HOST_FINGERPRINT
test "$(stat -c %a /mnt/etc/ssh/ssh_host_ed25519_key)" = 600
test "$(stat -c %u /mnt/etc/ssh/ssh_host_ed25519_key)" = 0
umount -R /mnt
test "$(lsblk --noheadings --output UUID /dev/vda2 | xargs)" = INSTALLATION_UUID
mount -o subvol=@root /dev/vda2 /mnt
mount -o subvol=@nix /dev/vda2 /mnt/nix
mount -o subvol=@var /dev/vda2 /mnt/var
mount -o subvol=@home /dev/vda2 /mnt/home
mount -o umask=0077 /dev/vda1 /mnt/boot
chmod 0755 /mnt/etc/ssh
echo NIX_SOURCE | base64 -d > /mnt/etc/nixos/bootstrap.nix
echo IDENTITY_SOURCE | base64 -d > /mnt/etc/nixos/identity.py
nixos-enter --root /mnt -c 'nixos-rebuild boot'
sync
""".replace("GUEST_UUID", record["plan"]["guest_uuid"]).replace("INSTALLATION_UUID", record["plan"]["installation_uuid"]).replace("HOST_FINGERPRINT", fingerprint).replace("NIX_SOURCE", source).replace("IDENTITY_SOURCE", identity_source)
    return encoded_console_script(script, "AIOS_REPAIR_EXIT")


def console_keys(text):
    plain = {" ": "spc", "-": "minus", "=": "equal", "/": "slash",
             ".": "dot", ",": "comma", ";": "semicolon", "'": "apostrophe", "\\": "backslash"}
    shifted = {">": "dot", "<": "comma", "&": "7", "_": "minus", ":": "semicolon",
               "$": "4", "%": "5", "+": "equal", "|": "backslash", "?": "slash", '"': "apostrophe"}
    result = []
    for character in text:
        if character in "abcdefghijklmnopqrstuvwxyz0123456789":
            keys = [character]
        elif character in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
            keys = ["shift", character.lower()]
        elif character in plain:
            keys = [plain[character]]
        elif character in shifted:
            keys = ["shift", shifted[character]]
        else:
            raise invalid("Unsupported character in registered bootstrap console operation")
        result.append(keys)
    return result


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
    load_record(config)
    with operation_lock(config.root):
        if action == "start":
            return start(config, display, bootstrap)
        if action == "console":
            return console(config)
        if action == "stop":
            return stop(config)
        raise invalid("Unknown lifecycle operation")


def qemu_arguments(config, record, display, *, bootstrap=True, qualification=None):
    serial = DISK_SERIAL
    if qualification is not None:
        from .acceptance import validate_disposable, WRONG_SERIAL
        if not bootstrap:
            raise invalid("Bootstrap qualification requires official installer media")
        validate_disposable(config, record, qualification)
        if qualification == "wrong-disk":
            serial = WRONG_SERIAL
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
    media = ["-drive", f"if=none,id=installer,media=cdrom,readonly=on,file={record['media']['path']}",
             "-device", "ide-cd,drive=installer,bus=ide.0,bootindex=1",
             "-drive", f"if=none,id=seed,media=cdrom,readonly=on,file={record['seed_iso']}",
             "-device", "ide-cd,drive=seed,bus=ide.1"] if bootstrap else []
    return [executable, "-name", config.values["name"], "-machine", "q35,accel=kvm", "-cpu", "host",
            "-smp", str(config.values["vcpus"]), "-m", str(config.values["memory_mib"]), "-uuid", record["plan"]["guest_uuid"],
            "-drive", f"if=pflash,format=raw,readonly=on,file={record['firmware_code']}",
            "-drive", f"if=pflash,format=raw,file={config.paths['nvram_file']}",
            "-drive", f"if=none,id=rootdisk,format=qcow2,file={config.paths['disk_image']}",
            "-device", f"virtio-blk-pci,drive=rootdisk,serial={serial},bootindex={2 if bootstrap else 1}",
            *media,
            "-netdev", f"user,id=net0,hostfwd=tcp:127.0.0.1:{config.values['ssh_port']}-:22", "-device", "virtio-net-pci,netdev=net0",
            "-device", "virtio-vga", "-display", display,
            "-qmp", f"unix:{config.paths['qmp_socket']},server=on,wait=off",
            "-chardev", f"socket,id=serial0,path={config.paths['serial_socket']},server=on,wait=off,logfile={config.paths['serial_log']}",
            "-serial", "chardev:serial0", "-boot", "menu=on", "-daemonize", "-pidfile", str(config.root / ".local/vm/qemu.pid")]


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
        if name not in ("qmp_capabilities", "query-uuid", "query-block", "query-status", "system_powerdown", "quit"):
            raise invalid("Unregistered QMP lifecycle operation")
        return self._request(name)

    def _request(self, name, arguments=None):
        verify_process(self.process)
        message = {"execute": name, "id": name}
        if arguments is not None:
            message["arguments"] = arguments
        self.socket.sendall((json.dumps(message) + "\n").encode())
        for _ in range(64):
            response = self.read()
            if response.get("id") == name:
                if "error" in response:
                    raise failure(ExitCode.OPERATION_FAILURE, "QMP_OPERATION_FAILED", f"QMP rejected {name}")
                return response["return"]
        raise failure(ExitCode.TIMEOUT, "QMP_TIMEOUT", "No matching lifecycle response")

    def capture(self, config):
        # Host display observation, with no guest command or keyboard input.
        verify_block(self, config)
        directory = private_directory(config.root, ".local/vm/console")
        path = directory / f"screen-{time.time_ns()}.png"
        self._request("screendump", {"filename": str(path), "format": "png"})
        path.chmod(0o600)
        return path

    def bootstrap_console(self, config, *, resume=False, finish=False, audit=False, repair=False, qualification=None):
        if sum((resume, finish, audit, repair, qualification is not None)) > 1:
            raise invalid("Choose one registered bootstrap console operation")
        verify_block(self, config)
        record = load_record(config)
        if f"if=none,id=installer,media=cdrom,readonly=on,file={record['media']['path']}" not in self.process["arguments"]:
            raise failure(ExitCode.TARGET_MISMATCH, "INSTALLER_CONSOLE_REQUIRED", "Bootstrap actions require the verified official installer console")
        command = BOOTSTRAP_CONSOLE
        operation = "repair" if repair else "audit" if audit else "finish" if finish else "bootstrap"
        if qualification is not None:
            from .acceptance import console_command, WRONG_SERIAL
            command = console_command(config, record, qualification)
            serial = WRONG_SERIAL if qualification == "wrong-disk" else DISK_SERIAL
            required = [f"virtio-blk-pci,drive=rootdisk,serial={serial},bootindex=2",
                        f"if=none,id=seed,media=cdrom,readonly=on,file={record['seed_iso']}"]
            if any(argument not in self.process["arguments"] for argument in required):
                raise failure(ExitCode.TARGET_MISMATCH, "QUALIFICATION_LAUNCH_MISMATCH", "Disposable launch does not match its registered case")
            operation = "qualification-" + qualification
        if repair:
            command = repair_console_command(config, record)
        if audit:
            from .guest import load_trust
            load_trust(config)
            command = audit_console_command(config, record)
        if finish:
            from .guest import load_trust
            load_trust(config)
            command = finish_console_command(load_record(config))
        if resume:
            command = command.replace("bash /run/aios-seed/bootstrap.sh;", "bash /run/aios-seed/bootstrap.sh --resume-unformatted;")
        payload = None
        if isinstance(command, tuple):
            command, payload = command
        sequence = [["ctrl", "c"], ["ctrl", "u"], *console_keys(command), ["ret"]]
        attempt = f"{self.process['pid']}-{time.time_ns()}" if audit or repair else str(self.process["pid"])
        marker = config.root / f".local/vm/{operation}-console-{attempt}.json"
        # Retain attempted state even on interrupted delivery. Never retry an
        # installer blindly after a partial command or possible disk mutation.
        write_json_new(marker, {"schema_version": 1, "state": "delivery-started",
                                "process": self.process, "operation": "recover-unformatted" if resume else operation,
                                "command_sha256": hashlib.sha256(command.encode()).hexdigest(),
                                "payload_sha256": hashlib.sha256(payload).hexdigest() if payload is not None else None})
        with serial_connection(config, self.process) as serial:
            directory = private_directory(config.root, ".local/vm/console")
            path = directory / f"{operation}-{attempt}.log"
            fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
            with os.fdopen(fd, "wb", buffering=0) as output:
                self._console_sequence(sequence)
                # QEMU accepts one QMP client on this endpoint. Release it while
                # the installer runs so read-only console captures stay usable.
                self.close()
                deadline = time.monotonic() + (300 if qualification is not None else 3600)
                ready_deadline = time.monotonic() + 45
                trailing = b""
                while time.monotonic() < deadline:
                    verify_process(self.process)
                    try:
                        chunk = serial.recv(65536)
                    except socket.timeout:
                        if payload is not None and time.monotonic() > ready_deadline:
                            raise failure(ExitCode.TIMEOUT, "CONSOLE_NOT_READY", "Installer did not acknowledge the registered operation; inspect retained evidence before retry")
                        continue
                    if not chunk:
                        raise failure(ExitCode.VERIFICATION_FAILURE, "BOOTSTRAP_CONSOLE_CLOSED", "Bootstrap serial channel closed before completion")
                    if output.tell() + len(chunk) > 64 * 1024 * 1024:
                        raise failure(ExitCode.VERIFICATION_FAILURE, "SERIAL_EVIDENCE_LIMIT", "Bootstrap evidence exceeds 64 MiB")
                    output.write(chunk)
                    trailing = (trailing + chunk)[-65536:]
                    if payload is not None and b"AIOS_CONSOLE_READY\n" in trailing:
                        serial.sendall(payload)
                        payload = None
                    sentinel = b"AIOS_QUALIFICATION_EXIT" if qualification is not None else b"AIOS_REPAIR_EXIT" if repair else b"AIOS_AUDIT_EXIT" if audit else b"AIOS_FINISH_EXIT" if finish else b"AIOS_BOOTSTRAP_EXIT"
                    match = re.search(rb"(?:^|[\r\n])" + sentinel + rb"=([0-9]+)[\r\n]", trailing)
                    if match:
                        status = int(match[1])
                        if status == 0 and not audit and not repair and qualification is None:
                            record = load_record(config)
                            receipt_name = "bootstrap-finish.json" if finish else "bootstrap-result.json"
                            write_json_new(config.root / ".local/vm" / receipt_name, {
                                "schema_version": 1, "upstream_exit": status,
                                "guest_uuid": record["plan"]["guest_uuid"],
                                "installation_uuid": record["plan"]["installation_uuid"],
                                "seed_sha256": record["seed_sha256"],
                                "media_sha256": record["media"]["sha256"],
                                "serial_path": str(path), "serial_sha256": digest_file(path),
                                "process": self.process,
                            })
                        state = "guard-qualified" if qualification is not None else "access-repaired" if repair else "audited" if audit else "setup-finished" if finish else "installer-finished"
                        return (ExitCode.SUCCESS if status == 0 else ExitCode.OPERATION_FAILURE), {"state": state if status == 0 else "bootstrap-failed", "upstream_exit": status, "artifact_path": str(path), "guest_identity_verified": False}
                raise failure(ExitCode.TIMEOUT, "BOOTSTRAP_TIMEOUT", "Registered console operation exceeded its deadline; no retry or disk reset attempted")

    def inspect_console(self, config):
        verify_block(self, config)
        self._console_sequence([["ctrl", "u"], *console_keys(BOOTSTRAP_INSPECT), ["ret"]])

    def _console_sequence(self, sequence):
        for keys in sequence:
            self._request("send-key", {"keys": [{"type": "qcode", "data": key} for key in keys], "hold-time": 10})
            time.sleep(0.03)

    def close(self):
        self.reader.close()
        self.socket.close()


def verify_block(client, config):
    blocks = client.command("query-block")
    matches = [b for b in blocks if b.get("device") == "rootdisk"]
    if len(matches) != 1 or matches[0].get("inserted", {}).get("file") != str(config.paths["disk_image"]):
        raise failure(ExitCode.TARGET_MISMATCH, "VM_DISK_MISMATCH", "QMP root disk does not match the configured virtual disk")


def start(config, display, bootstrap, *, qualification=None):
    pending = config.root / '.local/vm/restore-pending.json'
    if pending.exists() or pending.is_symlink():
        raise failure(ExitCode.VERIFICATION_FAILURE, 'RESTORE_INCOMPLETE', 'Complete the interrupted explicit restore before starting the VM')
    record = load_record(config)
    if not bootstrap:
        from .guest import load_trust
        trust = load_trust(config)
        receipt = read_json(project_path(config.root, ".local/vm/bootstrap-finish.json", ".local/vm"))
        if receipt.get("upstream_exit") != 0 or receipt.get("guest_uuid") != record["plan"]["guest_uuid"] or trust["expected"]["guest_uuid"] != record["plan"]["guest_uuid"]:
            raise failure(ExitCode.TARGET_MISMATCH, "INSTALLATION_NOT_FINISHED", "Installed startup requires verified cleanup and console SSH trust")
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
    args = qemu_arguments(config, record, display, bootstrap=bootstrap, qualification=qualification)
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
    return ExitCode.SUCCESS, {"state": "bootstrap-running" if bootstrap else "installed-running", "guest_identity_verified": False, "qmp_uuid_verified": True, "pid": pid, "qemu_status": status, "display": display, "message": "Verify/enroll guest SSH identity before guest operations."}


def console(config, *, capture=False, bootstrap_run=False, bootstrap_inspect=False, bootstrap_recover=False, bootstrap_finish=False, bootstrap_audit=False, bootstrap_repair=False, qualification=None):
    if sum((capture, bootstrap_run, bootstrap_inspect, bootstrap_recover, bootstrap_finish, bootstrap_audit, bootstrap_repair, qualification is not None)) > 1:
        raise invalid("Choose one registered console operation")
    record = load_record(config)
    process = read_json(project_path(config.root, ".local/vm/process.json", ".local/vm"))
    client = QMP(config, process, record["plan"]["guest_uuid"])
    try:
        verify_block(client, config)
        if capture:
            path = client.capture(config)
            return ExitCode.SUCCESS, {"guest_uuid": record["plan"]["guest_uuid"], "artifact_path": str(path), "guest_identity_verified": False}
        if bootstrap_run or bootstrap_recover or bootstrap_finish or bootstrap_audit or bootstrap_repair or qualification is not None:
            return client.bootstrap_console(config, resume=bootstrap_recover, finish=bootstrap_finish, audit=bootstrap_audit, repair=bootstrap_repair, qualification=qualification)
        if bootstrap_inspect:
            client.inspect_console(config)
            return ExitCode.SUCCESS, {"state": "bootstrap-inspection-submitted", "guest_identity_verified": False}
    finally:
        client.close()
    return ExitCode.SUCCESS, {"guest_uuid": record["plan"]["guest_uuid"], "message": "Inspect the installer with vm console --capture, then submit the registered host operation vm console --bootstrap-run. No manual guest sudo step is needed.", "guest_identity_verified": False}


def serial_connection(config, process):
    verify_process(process)
    serial = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        serial.settimeout(1)
        serial.connect(str(config.paths["serial_socket"]))
        pid, uid, _ = struct.unpack("3i", serial.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i")))
        if pid != process["pid"] or uid != os.getuid():
            raise failure(ExitCode.TARGET_MISMATCH, "SERIAL_PEER_MISMATCH", "Serial peer is not the verified QEMU process")
        return serial
    except BaseException:
        serial.close()
        raise


def follow_console(config, seconds):
    # Read-only local serial transport; it never sends console input. QEMU's
    # socket backend needs a connected reader while the guest transmits.
    record = load_record(config)
    process = read_json(project_path(config.root, ".local/vm/process.json", ".local/vm"))
    client = QMP(config, process, record["plan"]["guest_uuid"])
    try:
        verify_block(client, config)
    finally:
        client.close()
    directory = private_directory(config.root, ".local/vm/console")
    path = directory / "serial-capture.log"
    with serial_connection(config, process) as serial:
        fd = os.open(path, os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, "ab", buffering=0) as output:
            if not stat.S_ISREG(os.fstat(output.fileno()).st_mode):
                raise invalid("Serial evidence must be a regular file")
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                verify_process(process)
                try:
                    chunk = serial.recv(65536)
                except socket.timeout:
                    continue
                if not chunk:
                    break
                if output.tell() + len(chunk) > 64 * 1024 * 1024:
                    raise failure(ExitCode.VERIFICATION_FAILURE, "SERIAL_EVIDENCE_LIMIT", "Serial evidence exceeds 64 MiB")
                output.write(chunk)
    return ExitCode.SUCCESS, {"artifact_path": str(path), "guest_identity_verified": False}


def cleanup_stopped(config, process, state):
    # A missing PID is authoritative exit evidence, not a timed-out observation.
    # Never unlink a live or substituted control endpoint while reaping state.
    from .guest import private_file
    if (Path("/proc") / str(process["pid"])).exists():
        raise failure(ExitCode.TARGET_MISMATCH, "VM_STILL_PRESENT", "Recorded PID must disappear before control cleanup")
    private_file(state)
    if read_json(state) != process:
        raise failure(ExitCode.TARGET_MISMATCH, "PROCESS_RECORD_CHANGED", "Process record changed before cleanup")
    pidfile = config.root / ".local/vm/qemu.pid"
    if pidfile.exists():
        info = pidfile.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_nlink != 1 or info.st_mode & 0o022 or pidfile.read_text().strip() != str(process["pid"]):
            raise invalid("PID file changed before cleanup")
    for field in ("qmp_socket", "serial_socket"):
        endpoint = config.paths[field]
        if endpoint.exists():
            info = endpoint.lstat()
            if not stat.S_ISSOCK(info.st_mode) or info.st_uid != os.getuid():
                raise invalid("Control endpoint changed before cleanup")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as probe:
                probe.settimeout(1)
                try:
                    probe.connect(str(endpoint))
                except (ConnectionRefusedError, FileNotFoundError):
                    pass
                else:
                    raise failure(ExitCode.TARGET_MISMATCH, "LIVE_CONTROL_ENDPOINT", "Control endpoint still accepts connections")
    for path in (state, pidfile, config.paths["qmp_socket"], config.paths["serial_socket"]):
        if path.is_symlink():
            raise invalid("Control artifacts must not become symlinks")
        path.unlink(missing_ok=True)


def exiting_process(process):
    entry = Path("/proc") / str(process["pid"])
    try:
        fields = (entry / "stat").read_text().rsplit(")", 1)[1].split()
        return fields[0] == "Z" and int(fields[19]) == process["start_ticks"] and entry.stat().st_uid == process["uid"] == os.getuid()
    except (OSError, ValueError, IndexError):
        return False


def stop(config, *, graceful=False):
    # Host power control is tied to the exact QMP process/UUID/disk, and works
    # without SSH. It never executes a guest command or sends a process signal.
    record = load_record(config)
    state = project_path(config.root, ".local/vm/process.json", ".local/vm")
    process = read_json(state)
    if not (Path("/proc") / str(process["pid"])).exists():
        cleanup_stopped(config, process, state)
        return ExitCode.SUCCESS, {"state": "stopped", "guest_uuid": record["plan"]["guest_uuid"], "guest_identity_verified": False, "shutdown": "already-exited"}
    client = QMP(config, process, record["plan"]["guest_uuid"])
    try:
        verify_block(client, config)
        client.command("system_powerdown" if graceful else "quit")
    finally:
        client.close()
    for _ in range(600 if graceful else 30):
        try:
            verify_process(process)
        except DevctlError:
            # Only disappearance of this exact PID allows stale-artifact cleanup.
            if not (Path("/proc") / str(process["pid"])).exists():
                cleanup_stopped(config, process, state)
                return ExitCode.SUCCESS, {"state": "stopped", "guest_uuid": record["plan"]["guest_uuid"], "guest_identity_verified": False, "shutdown": "acpi" if graceful else "qmp-quit"}
            if not exiting_process(process):
                raise
        time.sleep(0.1)
    raise failure(ExitCode.TIMEOUT, "VM_STOP_TIMEOUT", "Recorded QEMU process did not exit; no force-off or signal escalation was attempted")
