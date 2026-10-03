#!/usr/bin/env python3
"""Registered read-only running-guest inventory; no credential contents."""
import json
import os
from pathlib import Path
from layout_audit import credential_boundary


def codex_process(name, arguments):
    def match(value):
        return Path(value).name.lower() in {"codex", "codex-app", "openai-codex"} or "@openai/codex/" in value.lower()
    return name.lower().startswith("codex") or any(match(value) for value in arguments)


def inventory(proc=Path("/proc")):
    records, races, unreadable = [], 0, []
    paths = sorted((path for path in proc.iterdir() if path.name.isdigit()), key=lambda path: int(path.name))
    if len(paths) > 16384:
        raise ValueError("process inventory exceeds bound")
    for path in paths:
        try:
            with (path / "comm").open("rb") as file:
                name = file.read(4097)
            with (path / "cmdline").open("rb") as file:
                arguments = file.read(262145)
            if len(name) > 4096 or len(arguments) > 262144:
                raise ValueError("process metadata exceeds bound")
            name = name.decode("utf-8", errors="strict").strip()
            values = [value.decode("utf-8", errors="strict") for value in arguments.split(b"\0") if value]
            if codex_process(name, values):
                raise ValueError("Codex process found in guest")
            records.append({"pid": int(path.name), "name": name})
        except (FileNotFoundError, ProcessLookupError):
            races += 1
        except PermissionError:
            unreadable.append(int(path.name))
    if unreadable:
        raise ValueError("process inventory incomplete")
    return {"processes": records, "checked_processes": len(records), "exited_during_inventory": races,
            "unreadable_processes": unreadable, "codex_processes_found": 0, "raw_arguments_returned": False}


def mount_inventory(contents):
    types, proc_options = set(), None
    for line in contents.splitlines():
        before, after = line.split(" - ", 1)
        fields, super_fields = before.split(), after.split()
        types.add(super_fields[0])
        if fields[4] == "/proc":
            if proc_options is not None or super_fields[0] != "proc":
                raise ValueError("ambiguous process filesystem")
            proc_options = set(fields[5].split(",") + super_fields[2].split(","))
    if proc_options is None or any(value.startswith("hidepid=") and value not in {"hidepid=0", "hidepid=off"} for value in proc_options):
        raise ValueError("process inventory visibility restricted")
    if {"9p", "virtiofs", "nfs", "nfs4", "cifs", "fuse.sshfs"} & types:
        raise ValueError("shared-host filesystem detected")
    return {"filesystem_types": sorted(types), "proc_mount_options": sorted(proc_options), "shared_host_filesystem_detected": False}


def main():
    if os.geteuid() == 0 or "ID=nixos" not in Path("/etc/os-release").read_text():
        raise ValueError("verified unprivileged NixOS job required")
    processes = inventory()
    if not any(item["pid"] == 1 and item["name"] == "systemd" for item in processes["processes"]):
        raise ValueError("system process namespace unavailable")
    mounts = mount_inventory(Path("/proc/self/mountinfo").read_text())
    print("AIOS_HOST_BOUNDARY=" + json.dumps({"schema_version": 1, "evidence_kind": "actual-running-nixos-guest-read-only-inventory",
          "process_inventory": processes, "mount_inventory": mounts,
          "credential_locations": credential_boundary(Path("/etc/passwd").read_text()),
          "limitations": ["Process names/arguments are inspected, but arguments and credential contents are never returned.",
                          "Unreadable credential roots require the separate guarded read-only root audit; they are not reported as absent."]}, sort_keys=True), flush=True)


if __name__ == "__main__":
    main()
