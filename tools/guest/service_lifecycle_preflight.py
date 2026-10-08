#!/usr/bin/env python3
"""Fixed root-only lifecycle qualification for the disposable desktop image."""
import json
import os
from pathlib import Path
import pwd
import stat
import subprocess
import socket
import time

SYSTEMCTL = "/run/current-system/sw/bin/systemctl"
LOGINCTL = "/run/current-system/sw/bin/loginctl"
MARKER = Path("/tmp/aios-service-lifecycle-request")
REPORT_DIR = Path("/run/aios-service-lifecycle")
REPORT = REPORT_DIR / "report.json"


def text(path):
    return Path(path).read_text().strip()


def identity():
    os_release = dict(line.split("=", 1) for line in Path("/etc/os-release").read_text().splitlines() if "=" in line)
    return {
        "schema_version": 1,
        "boot_id": text("/proc/sys/kernel/random/boot_id"),
        "current_system": str(Path("/run/current-system").resolve()),
        "disk_serial": text("/sys/class/block/vda/serial"),
        "dmi_uuid": text("/run/aios/dmi-uuid"),
        "guest_role": text("/etc/aios/guest-role"),
        "hostname": socket.gethostname(),
        "installation_uuid": text("/etc/aios/installation-uuid"),
        "machine_id": text("/etc/machine-id"),
        "management_channel": "ssh-development",
        "os_id": os_release["ID"].strip('\"'),
        "os_version": os_release["VERSION_ID"].strip('\"'),
    }


def command(expected, argv, check=True):
    if identity() != expected:
        raise RuntimeError("target identity changed before lifecycle operation")
    result = subprocess.run(argv, capture_output=True, text=True, timeout=30)
    if identity() != expected:
        raise RuntimeError("target identity changed after lifecycle operation")
    if check and result.returncode:
        raise RuntimeError(f"command failed: {argv}: {result.stdout} {result.stderr}")
    return result


def systemctl(user=None):
    if user is None:
        return [SYSTEMCTL]
    if user != "tester":
        raise RuntimeError("unregistered lifecycle test user")
    return [SYSTEMCTL, "--user", f"--machine={user}@.host"]


def properties(expected, unit, user=None):
    result = command(expected, [*systemctl(user), "show", unit, "--property=ActiveState", "--property=SubState",
                                "--property=MainPID", "--property=InvocationID"])
    return dict(line.split("=", 1) for line in result.stdout.splitlines() if "=" in line)


def policy(expected, unit, required, user=None):
    names = ["LoadState", *required]
    result = command(expected, [*systemctl(user), "show", unit, *[f"--property={name}" for name in names]])
    values = dict(line.split("=", 1) for line in result.stdout.splitlines() if "=" in line)
    if values.get("LoadState") != "loaded":
        raise RuntimeError(f"unit is not loaded: {unit}: {values}")
    unordered = {"RestrictAddressFamilies"}
    mismatches = {name: {"expected": value, "actual": values.get(name)}
                  for name, value in required.items()
                  if (set(values.get(name, "").split()) != set(value.split())
                      if name in unordered else values.get(name) != value)}
    if mismatches:
        raise RuntimeError(f"unit access policy mismatch: {unit}: {mismatches}")
    return {"unit": unit, "scope": "user" if user else "system", "properties": values}


def private_path(path, expected_uid, expected_gid, expected_mode, expected_kind):
    item = Path(path)
    value = item.lstat()
    kinds = {
        "directory": stat.S_ISDIR,
        "socket": stat.S_ISSOCK,
    }
    if (not kinds[expected_kind](value.st_mode) or value.st_uid != expected_uid
            or value.st_gid != expected_gid or stat.S_IMODE(value.st_mode) != expected_mode):
        raise RuntimeError(
            f"private path access mismatch: {path}: "
            f"uid={value.st_uid} gid={value.st_gid} mode={oct(stat.S_IMODE(value.st_mode))}"
        )
    if item.resolve() != item:
        raise RuntimeError(f"private path is not canonical: {path}")
    return {
        "path": path,
        "kind": expected_kind,
        "uid": value.st_uid,
        "gid": value.st_gid,
        "mode": oct(expected_mode),
    }


def access_plan(expected):
    state = pwd.getpwnam("aios-state")
    tester = pwd.getpwnam("tester")
    if state.pw_uid == 0 or state.pw_gid == 0 or state.pw_dir != "/var/empty":
        raise RuntimeError("graph owner is not an isolated service account")
    policies = [
        policy(expected, "aios-state.service", {
            "User": "aios-state", "Group": "aios-state", "NoNewPrivileges": "yes",
            "ProtectSystem": "strict", "ProtectHome": "tmpfs", "PrivateDevices": "yes",
            "PrivateNetwork": "no", "RestrictAddressFamilies": "AF_UNIX AF_NETLINK",
            "RestrictNamespaces": "yes", "MemoryDenyWriteExecute": "yes",
        }),
        policy(expected, "aios-execd.service", {
            "User": "root", "Group": "root", "NoNewPrivileges": "yes",
            "ProtectSystem": "strict", "ProtectHome": "tmpfs", "PrivateNetwork": "yes",
            "RestrictAddressFamilies": "AF_UNIX", "RestrictNamespaces": "yes",
            "MemoryDenyWriteExecute": "yes",
        }),
        policy(expected, "aios-sessiond.service", {
            "NoNewPrivileges": "yes", "ProtectSystem": "strict", "ProtectHome": "tmpfs",
            "PrivateDevices": "yes", "PrivateNetwork": "yes",
            "RestrictAddressFamilies": "AF_UNIX", "RestrictNamespaces": "yes",
            "MemoryDenyWriteExecute": "yes",
        }, "tester"),
        policy(expected, "aios-processd.service", {
            "NoNewPrivileges": "yes", "RestrictAddressFamilies": "AF_UNIX",
            "RestrictNamespaces": "yes", "MemoryDenyWriteExecute": "yes",
        }, "tester"),
        policy(expected, "aios-ui-agent.service", {
            "NoNewPrivileges": "yes", "RestrictAddressFamilies": "AF_UNIX",
            "RestrictNamespaces": "yes", "MemoryDenyWriteExecute": "yes",
        }, "tester"),
    ]
    paths = [
        private_path("/run/aios-state", state.pw_uid, state.pw_gid, 0o700, "directory"),
        private_path("/run/aios-state/owner.sock", state.pw_uid, state.pw_gid, 0o600, "socket"),
        private_path("/var/lib/aios/state", state.pw_uid, state.pw_gid, 0o700, "directory"),
        private_path(f"/run/user/{tester.pw_uid}/aios", tester.pw_uid, tester.pw_gid, 0o700, "directory"),
        private_path(f"/run/user/{tester.pw_uid}/aios/session.sock", tester.pw_uid, tester.pw_gid, 0o600, "socket"),
        private_path(f"/run/user/{tester.pw_uid}/aios-process", tester.pw_uid, tester.pw_gid, 0o700, "directory"),
        private_path(f"/run/user/{tester.pw_uid}/aios-process/provider.sock", tester.pw_uid, tester.pw_gid, 0o600, "socket"),
        private_path(f"/run/user/{tester.pw_uid}/aios-ui", tester.pw_uid, tester.pw_gid, 0o700, "directory"),
        private_path(f"/run/user/{tester.pw_uid}/aios-ui/provider.sock", tester.pw_uid, tester.pw_gid, 0o600, "socket"),
    ]
    return {
        "accounts": {"aios-state": {"uid": state.pw_uid, "gid": state.pw_gid, "home": state.pw_dir}},
        "unit_policies": policies,
        "private_paths": paths,
        "documented_exceptions": {
            "aios-state.service": "Host network namespace only for fixed read-only udev netlink subscription; IP address families denied.",
            "aios-execd.service": "Root broker has only fixed typed transaction routes and two bounded capabilities; writes are limited by ReadWritePaths.",
            "aios-processd.service": "Original user namespace required for native peer process identity; fixed own-user process route only.",
            "aios-ui-agent.service": "Original user namespace required for native desktop process identity; fixed graphical routes only.",
        },
    }


def restart(expected, unit, user=None):
    before = properties(expected, unit, user)
    if before["ActiveState"] != "active" or not before["MainPID"].isdigit() or int(before["MainPID"]) <= 1:
        raise RuntimeError(f"unit was not active before restart: {unit}: {before}")
    command(expected, [*systemctl(user), "restart", unit])
    after = properties(expected, unit, user)
    if after["ActiveState"] != "active" or not after["MainPID"].isdigit() or int(after["MainPID"]) <= 1:
        raise RuntimeError(f"unit failed to restart: {unit}: {after}")
    if before["InvocationID"] == after["InvocationID"] or before["MainPID"] == after["MainPID"]:
        raise RuntimeError(f"unit identity did not change: {unit}")
    return {"unit": unit, "before": before, "after": after}


def infrastructure(expected):
    values = {}
    for unit in ("sshd.service", "NetworkManager.service", "display-manager.service"):
        current = properties(expected, unit)
        if current["ActiveState"] != "active":
            raise RuntimeError(f"infrastructure service failed during AI lifecycle test: {unit}")
        values[unit] = current
    return values


def publish(value):
    REPORT_DIR.mkdir(mode=0o755, exist_ok=True)
    temporary = REPORT.with_suffix(".tmp")
    temporary.write_text(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")
    os.chmod(temporary, 0o644)
    os.replace(temporary, REPORT)

def main():
    if os.geteuid() != 0:
        raise RuntimeError("service lifecycle preflight requires root")
    expected = identity()
    if expected["guest_role"] != "development" or expected["management_channel"] != "ssh-development" or expected["disk_serial"] != "AIOS_DEV_ROOT":
        raise RuntimeError("service lifecycle preflight requires the enrolled disposable development guest")
    marker = MARKER.lstat()
    dev_uid = pwd.getpwnam("dev").pw_uid
    if not stat.S_ISREG(marker.st_mode) or marker.st_uid != dev_uid or stat.S_IMODE(marker.st_mode) != 0o600 or marker.st_size != 33:
        raise RuntimeError("invalid lifecycle request marker")
    token = MARKER.read_text()
    if len(token) != 33 or not token.endswith("\n") or any(c not in "0123456789abcdef" for c in token[:-1]):
        raise RuntimeError("invalid lifecycle request token")
    REPORT_DIR.mkdir(mode=0o755, exist_ok=True)
    system_restarts = []
    user_restarts = []
    infrastructure_before = infrastructure(expected)
    for unit in ("aios-state.service", "aios-execd.service"):
        system_restarts.append(restart(expected, unit))
        infrastructure(expected)
    model = command(expected, [SYSTEMCTL, "show", "aios-model.service", "--property=LoadState", "--property=ActiveState"], check=False)
    if model.returncode == 0 and "ActiveState=active" in model.stdout:
        raise RuntimeError("model service active in model-independent image")
    uid = pwd.getpwnam("tester").pw_uid
    for unit in ("aios-sessiond.service", "aios-processd.service", "aios-ui-agent.service"):
        user_restarts.append(restart(expected, unit, "tester"))
        infrastructure(expected)
    access = access_plan(expected)
    command(expected, [SYSTEMCTL, "stop", "display-manager.service"])
    logged_out = False
    try:
        command(expected, [LOGINCTL, "terminate-user", "tester"])
        deadline = time.monotonic() + 15
        while properties(expected, f"user@{uid}.service")["ActiveState"] != "inactive" and time.monotonic() < deadline:
            time.sleep(0.1)
        logged_out = properties(expected, f"user@{uid}.service")["ActiveState"] == "inactive"
        if not logged_out:
            raise RuntimeError("tester user manager survived logout")
        for unit in ("sshd.service", "NetworkManager.service"):
            if properties(expected, unit)["ActiveState"] != "active":
                raise RuntimeError(f"management service failed across logout: {unit}")
    finally:
        command(expected, [SYSTEMCTL, "start", "display-manager.service"], check=False)
    final_infrastructure = infrastructure(expected)
    result = {
        "schema_version": 1,
        "evidence_kind": "actual-installed-fixed-root-service-lifecycle-fixture",
        "verified": True,
        "request_token": token.strip(),
        "identity": expected,
        "system_restarts": system_restarts,
        "user_restarts": user_restarts,
        "access_plan": access,
        "model_active": False,
        "tester_runtime_removed": logged_out,
        "infrastructure_before": infrastructure_before,
        "infrastructure_after": final_infrastructure,
        "limits": ["Disposable desktop-test image only; fixture absent from production composition."],
    }
    publish(result)
    MARKER.unlink()


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        token = None
        try:
            value = MARKER.read_text()
            if len(value) == 33 and value.endswith("\n") and all(c in "0123456789abcdef" for c in value[:-1]):
                token = value.strip()
        except OSError:
            pass
        publish({
            "schema_version": 1,
            "evidence_kind": "actual-installed-fixed-root-service-lifecycle-fixture",
            "verified": False,
            "request_token": token,
            "error": {"kind": type(error).__name__, "message": str(error)[:512]},
        })
        raise
