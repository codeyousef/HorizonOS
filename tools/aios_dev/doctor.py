"""Read-only Linux host probes. TCP reachability never establishes guest identity."""
import os
import platform
import re
import shlex
import shutil
import socket
import stat
import subprocess
from pathlib import Path

from .config import VMConfig

TOOL_ARGUMENTS = {
    "python3": ["--version"], "ssh": ["-V"], "sftp": ["-h"],
    "qemu-system-x86_64": ["--version"], "qemu-img": ["--version"],
    "systemd-run": ["--version"], "xorriso": ["-version"],
}
FIRMWARE_DIRECTORIES = (
    Path("/usr/share/edk2/x64"), Path("/usr/share/edk2-ovmf/x64"), Path("/usr/share/OVMF"),
)


def tool_info(name: str) -> dict:
    path = shutil.which(name)
    if path is None:
        return {"available": False, "path": None, "version": None}
    try:
        result = subprocess.run(
            [path, *TOOL_ARGUMENTS[name]], capture_output=True, text=True,
            errors="replace", timeout=3, check=False,
        )
        lines = (result.stdout + result.stderr).strip().splitlines()
        return {
            "available": result.returncode == 0 or name == "sftp",
            "path": path, "version": lines[0][:512] if lines else None,
            "upstream_exit": result.returncode,
        }
    except (OSError, subprocess.TimeoutExpired) as error:
        return {"available": False, "path": path, "version": None, "error": type(error).__name__}


def os_info(path: Path = Path("/etc/os-release")) -> dict:
    values = {}
    try:
        for line in path.read_text().splitlines():
            if "=" not in line or line.startswith("#"):
                continue
            key, value = line.split("=", 1)
            tokens = shlex.split(value)
            values[key] = " ".join(tokens)
    except (OSError, ValueError):
        pass
    return {"id": values.get("ID"), "name": values.get("PRETTY_NAME"), "version": values.get("VERSION_ID")}


def memory_info(path: Path = Path("/proc/meminfo")) -> dict:
    values = {}
    try:
        for line in path.read_text().splitlines():
            key, value = line.split(":", 1)
            parts = value.split()
            if parts and parts[0].isdigit() and len(parts) > 1 and parts[1] == "kB":
                values[key] = int(parts[0]) * 1024
    except (OSError, ValueError):
        pass
    return {"total_bytes": values.get("MemTotal"), "available_bytes": values.get("MemAvailable")}


def execution_context() -> str:
    path = shutil.which("systemd-detect-virt")
    if path is None:
        return "unknown"
    try:
        result = subprocess.run([path], capture_output=True, text=True, timeout=2, check=False)
        return result.stdout.strip()[:128] or "unknown"
    except (OSError, subprocess.TimeoutExpired):
        return "unknown"


def kvm_info(path: Path = Path("/dev/kvm")) -> dict:
    exists = path.exists()
    return {"exists": exists, "read_write_access": exists and os.access(path, os.R_OK | os.W_OK)}


def qemu_capabilities() -> dict:
    path = shutil.which("qemu-system-x86_64")
    result = {"virtio_vga": False, "gtk": False, "probe_complete": False}
    if not path:
        return result
    try:
        devices = subprocess.run([path, "-device", "help"], capture_output=True, text=True, timeout=3, check=False)
        displays = subprocess.run([path, "-display", "help"], capture_output=True, text=True, timeout=3, check=False)
        result.update({"virtio_vga": bool(re.search(r'^name "virtio-vga"(?:,|$)', devices.stdout, re.MULTILINE)), "gtk": "gtk" in displays.stdout.splitlines(), "probe_complete": devices.returncode == 0 and displays.returncode == 0})
    except (OSError, subprocess.TimeoutExpired):
        pass
    return result


def find_firmware(directories=FIRMWARE_DIRECTORIES) -> dict:
    # A CODE/VARS pair must belong to the same format, not arbitrary matching files.
    for directory in directories:
        for code_name, vars_name in (
            ("OVMF_CODE.4m.fd", "OVMF_VARS.4m.fd"), ("OVMF_CODE.fd", "OVMF_VARS.fd"),
        ):
            code, variables = directory / code_name, directory / vars_name
            if code.is_file() and variables.is_file() and os.access(code, os.R_OK) and os.access(variables, os.R_OK):
                return {"available": True, "code": str(code), "variables_template": str(variables)}
    return {"available": False, "code": None, "variables_template": None}


def tcp_probe(host: str, port: int) -> dict:
    try:
        with socket.create_connection((host, port), timeout=0.5):
            return {"reachable": True, "identity_verified": False}
    except ConnectionRefusedError:
        return {"reachable": False, "identity_verified": False, "reason": "connection_refused"}
    except (OSError, TimeoutError) as error:
        # A timeout or inaccessible network cannot prove that the port is free.
        return {"reachable": None, "identity_verified": False, "reason": type(error).__name__}


def qemu_processes(config: VMConfig, proc: Path = Path("/proc")) -> dict:
    matches = []
    try:
        pids = [p for p in proc.iterdir() if p.name.isdecimal()]
    except OSError:
        return {"matches": [], "complete": False}
    for entry in pids[:4096]:
        try:
            if not (entry / "comm").read_text().strip().startswith("qemu-system"):
                continue
            args = (entry / "cmdline").read_bytes().split(b"\x00")
            qmp = f"unix:{config.paths['qmp_socket']},server=on,wait=off".encode()
            disk = str(config.paths["disk_image"]).encode()
            has_qmp = any(a == qmp for a in args)
            has_disk = any(segment == b"file=" + disk for a in args for segment in a.split(b","))
            forward = f"hostfwd=tcp:{config.values['ssh_host']}:{config.values['ssh_port']}-:22".encode()
            has_forward = any(forward == segment for a in args for segment in a.split(b","))
            if has_qmp or has_disk:
                fields = (entry / "stat").read_text().rsplit(")", 1)[1].split()
                matches.append({"pid": int(entry.name), "start_ticks": int(fields[19]), "qmp_matches": has_qmp, "disk_matches": has_disk,
                                "forward_matches": has_forward, "same_user": entry.stat().st_uid == os.getuid(), "identity_verified": False})
        except (OSError, ValueError, IndexError):
            continue
    return {"matches": matches, "complete": len(pids) <= 4096}


def ssh_file_permissions(config: VMConfig) -> list[dict]:
    problems = []
    for key in ("identity_file", "known_hosts_file"):
        path = config.paths[key]
        if not path.exists():
            continue
        try:
            mode = stat.S_IMODE(path.stat().st_mode)
            parent_mode = stat.S_IMODE(path.parent.stat().st_mode)
            if mode & 0o077 or parent_mode & 0o077:
                problems.append({"field": key, "reason": "SSH files require private file and directory permissions"})
        except OSError as error:
            problems.append({"field": key, "reason": type(error).__name__})
    return problems


def host_report(config: VMConfig) -> dict:
    tools = {name: tool_info(name) for name in TOOL_ARGUMENTS}
    firmware, kvm = find_firmware(), kvm_info()
    disk = shutil.disk_usage(config.root)
    memory = memory_info()
    cpu_count = os.cpu_count()
    processes = qemu_processes(config)
    capabilities = qemu_capabilities() if config.values["provider"] == "qemu" else None
    # Probe only the configured loopback port by default, never scan other ports.
    probe = tcp_probe(config.values["ssh_host"], config.values["ssh_port"]) if config.values["provider"] == "qemu" or config.configured else {"reachable": None, "identity_verified": False, "reason": "not_configured"}
    missing = []
    def require(ready, prerequisite, operations):
        if not ready:
            missing.append({"prerequisite": prerequisite, "affected_operations": operations})
    require(tools["python3"]["available"], "Python 3", ["all host commands"])
    require(tools["ssh"]["available"], "OpenSSH", ["guest discovery", "guest operations"])
    require(tools["sftp"]["available"], "OpenSSH SFTP", ["source sync", "artifact transfer"])
    if config.values["provider"] == "qemu":
        require(platform.machine() == "x86_64", "x86-64 host for the KVM target", ["vm create", "vm start", "CPU benchmarks"])
        require(tools["qemu-system-x86_64"]["available"], "QEMU x86-64", ["vm start", "acceptance guests"])
        require(tools["systemd-run"]["available"], "systemd-run user service launcher", ["vm start", "acceptance guests"])
        require(capabilities["probe_complete"] and capabilities["virtio_vga"], "QEMU virtio-vga display device", ["vm start"])
        require(capabilities["probe_complete"] and capabilities["gtk"], "QEMU local GTK display backend", ["vm start --display gtk"])
        require(tools["qemu-img"]["available"], "qemu-img", ["vm create", "cold snapshots"])
        require(tools["xorriso"]["available"], "xorriso", ["read-only seed ISO"])
        require(firmware["available"], "matching OVMF CODE/VARS files", ["UEFI vm create"])
        require(kvm["read_write_access"], "read/write /dev/kvm access", ["KVM vm start", "CPU benchmarks"])
        if not any(p["qmp_matches"] and p["disk_matches"] and p["forward_matches"] and p["same_user"] for p in processes["matches"]):
            require(probe["reachable"] is False, "unoccupied, verifiable SSH forwarding port", ["vm start"])
        require(cpu_count is not None and config.values["vcpus"] <= cpu_count, "enough logical CPUs for configured vcpus", ["vm start"])
        require(memory["available_bytes"] is not None and config.values["memory_mib"] * 1024**2 <= memory["available_bytes"], "enough available RAM for configured memory_mib", ["vm start"])
        require(config.values["disk_gib"] * 1024**3 <= disk.free, "disk headroom for configured disk_gib", ["vm create"])
    permissions = ssh_file_permissions(config)
    require(not permissions, "private SSH file/directory permissions", ["guest authentication"])
    return {
        "read_only": True, "os": os_info(), "architecture": platform.machine(),
        "kernel": platform.release(), "execution_context": execution_context(),
        "logical_cpus": cpu_count, "memory": memory,
        "disk": {"workspace": str(config.root), "free_bytes": disk.free, "total_bytes": disk.total},
        "tools": tools, "kvm": kvm, "firmware": firmware, "qemu_capabilities": capabilities,
        "configuration": {"local": config.configured, "provider": config.values["provider"], "name": config.values["name"]},
        "ssh_port": {"host": config.values["ssh_host"], "port": config.values["ssh_port"], "available_for_forwarding": probe["reachable"] is False, **probe},
        "guest": {"state": "not_verified", "identity_verified": False, "ssh_tcp_reachable": probe["reachable"]},
        "workspace_qemu": processes, "permission_problems": permissions,
        "missing_prerequisites": missing,
    }
