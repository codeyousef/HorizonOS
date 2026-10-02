#!/usr/bin/env python3
"""Unprivileged read-only identity endpoint, executed only in a NixOS guest."""
import json
from pathlib import Path
import shlex
import socket


def text(path):
    return Path(path).read_text(encoding="utf-8").strip()


def main():
    os_release = {}
    for line in text("/etc/os-release").splitlines():
        if "=" in line and not line.startswith("#"):
            name, value = line.split("=", 1)
            os_release[name] = " ".join(shlex.split(value))
    print(json.dumps({
        "schema_version": 1, "os_id": os_release["ID"], "os_version": os_release["VERSION_ID"],
        "hostname": socket.gethostname(), "dmi_uuid": text("/run/aios/dmi-uuid"),
        "installation_uuid": text("/etc/aios/installation-uuid"), "guest_role": text("/etc/aios/guest-role"),
        "boot_id": text("/proc/sys/kernel/random/boot_id"), "machine_id": text("/etc/machine-id"),
        "current_system": str(Path("/run/current-system").resolve(strict=True)),
        "disk_serial": text("/sys/class/block/vda/serial"), "management_channel": "ssh-development",
    }, sort_keys=True))


if __name__ == "__main__":
    main()
