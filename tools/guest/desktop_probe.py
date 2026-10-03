#!/usr/bin/env python3
"""Fixed read-only probe of the synthetic disposable Plasma session."""
import json
from pathlib import Path
import pwd
import subprocess


def command(arguments):
    return subprocess.run(arguments, capture_output=True, text=True, check=True, timeout=5).stdout


def main():
    profile = Path("/etc/aios/desktop-test-profile").read_text().strip()
    if profile != "synthetic-disposable-plasma-wayland-v1":
        raise ValueError("Wrong desktop image")
    uid = pwd.getpwnam("tester").pw_uid
    sessions = []
    for line in command(["/run/current-system/sw/bin/loginctl", "list-sessions", "--no-legend", "--no-pager"]).splitlines():
        session = line.split()[0]
        properties = command(["/run/current-system/sw/bin/loginctl", "show-session", session, "--no-pager",
                              "-p", "User", "-p", "Name", "-p", "Type", "-p", "Class", "-p", "Active", "-p", "Remote", "-p", "State"]).splitlines()
        value = dict(item.split("=", 1) for item in properties)
        if value == {"User": str(uid), "Name": "tester", "Type": "wayland", "Class": "user", "Active": "yes", "Remote": "no", "State": "active"}:
            sessions.append(session)
    processes = []
    expected = {str(Path("/run/current-system/sw/bin").joinpath(name).resolve(strict=True)): name for name in ("kwin_wayland", "plasmashell")}
    # Nix wrappers change Linux comm (which is also truncated to 15 bytes).
    # Match the full argv[0] to this running closure's package links instead.
    for line in command(["/run/current-system/sw/bin/ps", "-u", "tester", "-o", "pid=,uid=,args="]).splitlines():
        pid, owner, arguments = line.split(maxsplit=2)
        executable = arguments.split(" ", 1)[0]
        if int(owner) == uid and executable in expected:
            processes.append({"pid": int(pid), "uid": uid, "name": expected[executable], "executable": executable})
    if len(sessions) != 1 or {item["name"] for item in processes} != {"kwin_wayland", "plasmashell"}:
        raise ValueError("Synthetic Wayland session is not ready")
    print(json.dumps({"schema_version": 1, "profile": profile, "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
                      "tester_uid": uid, "session_id": sessions[0], "processes": processes}, sort_keys=True))


if __name__ == "__main__":
    main()
