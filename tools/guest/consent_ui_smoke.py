#!/usr/bin/env python3
"""Native Qt fixture tests and real dialog fail-closed transport checks.

This is a registered disposable desktop scenario, not an approval issuer. It
never produces a policy capability or uses a model/computer-use activation.
"""
import hashlib
import json
import os
from pathlib import Path
import socket
import stat
import struct
import subprocess
import time


def main():
    observed = json.loads(subprocess.run(["python3",str(Path(__file__).with_name("desktop_probe.py"))],check=True,stdout=subprocess.PIPE,timeout=30).stdout)
    uid = os.geteuid()
    desktop = observed
    if uid != desktop["tester_uid"]:
        raise RuntimeError("native GUI scenario requires the real tester UID")
    runtime = Path("/run/user/" + str(uid))
    info = runtime.lstat()
    if runtime.resolve() != runtime or info.st_uid != uid or not stat.S_ISDIR(info.st_mode) or info.st_mode & 0o077:
        raise RuntimeError("unsafe native user runtime")
    # Fixture-only uniqueness rule. Production must retain explicit selection
    # and native display association; this probe does not mint that authority.
    displays = [p for p in sorted(runtime.glob("wayland-*")) if stat.S_ISSOCK(p.lstat().st_mode)]
    if len(displays) != 1:
        raise RuntimeError("fixture display is ambiguous")
    display = displays[0]
    with socket.socket(socket.AF_UNIX) as connection:
        connection.settimeout(2)
        connection.connect(str(display))
        compositor_pid, compositor_uid, _ = struct.unpack("3i", connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        if compositor_uid != uid or compositor_pid <= 1:
            raise RuntimeError("wrong native compositor owner")
    release = Path(__file__).resolve().parents[2]
    build = ["nix", "build", "--json", "--no-link", "--no-update-lock-file", "--no-write-lock-file",
             "path:" + str(release) + "#aios-consent-ui", "path:" + str(release) + "#checks.x86_64-linux.consent-ui"]
    print("CONSENT_BUILD=" + json.dumps(build), flush=True)
    outputs = json.loads(subprocess.run(build, check=True, stdout=subprocess.PIPE, timeout=900).stdout)
    paths = [Path(o["outputs"]["out"]) for o in outputs]
    production = next(p for p in paths if p.name.endswith("-aios-consent-ui-0.1.0"))
    tests = next(p for p in paths if p.name.endswith("-aios-consent-ui-tests-0.1.0"))
    if (production / "libexec/consent-ui-test").exists():
        raise RuntimeError("test activation binary leaked into production UI output")
    env = {"PATH": "/run/current-system/sw/bin", "XDG_RUNTIME_DIR": str(runtime), "WAYLAND_DISPLAY": display.name,
           "DBUS_SESSION_BUS_ADDRESS": "unix:path=" + str(runtime / "bus"), "QT_QPA_PLATFORM": "wayland", "LANG": "C.UTF-8"}
    result = subprocess.run([str(tests / "libexec/consent-ui-test")], env=env, check=True, stdout=subprocess.PIPE, timeout=30)
    print("NATIVE_WAYLAND_WIDGET_FIXTURES=" + result.stdout.decode(), flush=True)
    def fixture(lifetime):
        now = time.clock_gettime_ns(time.CLOCK_BOOTTIME) // 1000000
        return {"schema_version": 1, "kind": "read_scope", "digest": "a" * 64, "uid": uid,
                "session_id": desktop["session_id"], "target": "This disposable VM", "profile": "Fixture, no model running", "mode": "ask",
                "goal": "Explain the named fixture document", "apps": [{"handle": "fixture-app", "identity_sha256": "b" * 64, "name": "Kate", "window": "Fixture document"}],
                "actions": ["ui.snapshot"], "issued_ms": now, "expires_ms": now + lifetime, "evidence": ["synthetic-scope-fixture"]}
    def run(document, withdraw=False, extra=b""):
        data = json.dumps(document, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
        parent, child = socket.socketpair()
        parent.settimeout(5)
        process = subprocess.Popen([str(production / "bin/aios-scope-dialog")], stdin=child, stdout=subprocess.DEVNULL,
                                   stderr=subprocess.PIPE, env=env)
        child.close()
        start = time.monotonic()
        try:
            parent.sendall(struct.pack(">I", len(data)) + data)
            if withdraw or extra:
                time.sleep(0.35)
                start = time.monotonic()
                if withdraw:
                    parent.shutdown(socket.SHUT_WR)
                else:
                    parent.sendall(extra)
            received = parent.recv(4096)
            process.wait(timeout=5)
            if process.returncode != 0:
                raise RuntimeError("real dialog failed: " + process.stderr.read(4096).decode())
            reply = json.loads(received)
            if reply != {"digest": document["digest"], "decision": "cancel"}:
                raise RuntimeError("real dialog granted fixture authority")
            return {"exit": process.returncode, "decision": reply["decision"], "elapsed_ms": int((time.monotonic()-start)*1000)}
        finally:
            parent.close()
            if process.poll() is None:
                process.terminate()
            process.wait(timeout=5)
            process.stderr.close()
    expiry = run(fixture(800))
    withdrawal = run(fixture(10000), withdraw=True)
    changed = run(fixture(10000), extra=b"changed proposal")
    if max(withdrawal["elapsed_ms"], changed["elapsed_ms"]) > 1000:
        raise RuntimeError("native withdrawal was not responsive")
    cli = subprocess.run([str(production / "bin/aios-scope-dialog"), "--approve"], stdin=subprocess.DEVNULL,
                         env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=3)
    if cli.returncode != 2 or cli.stdout:
        raise RuntimeError("automated approval argument was accepted")
    print("CONSENT_NATIVE_RESULT=" + json.dumps({"evidence_kind": "real-native-dialog-with-synthetic-proposal-no-policy-grant",
        "desktop": desktop, "compositor_pid": compositor_pid, "uid": uid, "platform": "wayland", "outputs": [str(p) for p in paths],
        "production_binary_sha256": hashlib.sha256((production / "bin/.aios-scope-dialog-wrapped").read_bytes()).hexdigest(),
        "expiry": expiry, "withdrawal": withdrawal, "changed_proposal": changed, "approval_argument_exit": cli.returncode}), flush=True)


if __name__ == "__main__":
    main()
