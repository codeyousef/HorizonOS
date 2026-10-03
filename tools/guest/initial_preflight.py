#!/usr/bin/env python3
"""Fixed initial-development-image root preflight; no RPC, plans or effects."""
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys


def main():
    if os.getuid() != 0 or os.geteuid() != 0:
        print('{"schema_version":1,"error":"INITIAL_PREFLIGHT_ROOT_REQUIRED"}')
        return 5
    if len(sys.argv) != 1:
        return 2
    destination = Path("/run/aios-initial-preflight/result.json")
    parent = destination.parent.stat()
    if not stat.S_ISDIR(parent.st_mode) or parent.st_uid != 0 or parent.st_mode & 0o022:
        return 8
    report = {"schema_version":1,"evidence_kind":"actual-installed-root-native-preflight",
        "uid":os.getuid(),"effective_uid":os.geteuid(),"native_preflight_verified":False,
        "authenticated_caller_verified":False,"native_authorization_verified":False,
        "trusted_confirmation_verified":False,"product_effects_performed":False}
    try:
        executable = Path("/run/current-system/sw/bin/aios-execd").resolve(strict=True)
        info = executable.stat()
        if not executable.is_relative_to("/nix/store") or not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o222:
            raise ValueError("unsafe installed executable")
        report.update(executable=str(executable), executable_sha256=hashlib.sha256(executable.read_bytes()).hexdigest(),
            boot_id=Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            installation_uuid=Path("/etc/aios/installation-uuid").read_text().strip())
        # Capture the actual service sandbox rather than the SSH observer's
        # different mount namespace. These fixed public observations grant no
        # authority and expose no root/tester credentials.
        mounts = Path("/proc/self/mountinfo").read_text()
        if len(mounts) > 1024 * 1024:
            raise ValueError("mount observation exceeds limit")
        report["root_mountinfo"] = [line for line in mounts.splitlines()
            if len(line.split()) > 4 and line.split()[4] == "/"]
        report["uid_map"] = Path("/proc/self/uid_map").read_text().strip()
        result = subprocess.run([str(executable)], capture_output=True, timeout=15, check=False)
        report.update(argv=[str(executable)], upstream_exit=result.returncode)
        if len(result.stdout) > 16384 or len(result.stderr) > 16384:
            raise ValueError("preflight output limit")
        value = json.loads(result.stdout)
        report["native_result"] = value
        report["native_preflight_verified"] = result.returncode == 9 and value == {
            "schema_version":1,"error":"BROKER_RUNTIME_ADAPTER_UNAVAILABLE"}
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        report["failure"] = type(error).__name__
    descriptor = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o644)
    with os.fdopen(descriptor, "w") as handle:
        json.dump(report, handle, sort_keys=True)
        handle.write("\n")
        handle.flush()
        os.fsync(handle.fileno())
    return 0 if report["native_preflight_verified"] else 6


if __name__ == "__main__":
    raise SystemExit(main())
