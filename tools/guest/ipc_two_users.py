#!/usr/bin/env python3
"""Fixed two-real-UID IPC qualification in the disposable desktop image only.

The deliberately exposed endpoint is an isolated test socket. Installed broker
permissions are never changed. No caller code, credentials or approval is used.
"""
import errno
import json
import os
from pathlib import Path
import pwd
import socket
import stat
import struct
import subprocess
import sys
import time
import uuid

PROFILE = "synthetic-disposable-plasma-wayland-v1"
LIMIT = 65536


def subject(role):
    profile = Path("/etc/aios/desktop-test-profile")
    if profile.lstat().st_uid != 0 or profile.read_text().strip() != PROFILE:
        raise RuntimeError("not the disposable test image")
    value = json.loads(subprocess.check_output(["/run/current-system/sw/bin/aios-guest-identity"], timeout=5))
    for key, expected in {"os_id":"nixos", "guest_role":"development", "disk_serial":"AIOS_DEV_ROOT", "management_channel":"ssh-development",
                          "dmi_uuid":Path("/etc/aios/expected-dmi-uuid").read_text().strip(),
                          "installation_uuid":Path("/etc/aios/installation-uuid").read_text().strip(),
                          "boot_id":Path("/proc/sys/kernel/random/boot_id").read_text().strip()}.items():
        if value[key] != expected:
            raise RuntimeError("test target mismatch: " + key)
    dev, tester = pwd.getpwnam("dev").pw_uid, pwd.getpwnam("tester").pw_uid
    if dev == tester or min(dev, tester) < 1000 or os.geteuid() != (dev if role == "owner" else tester):
        raise RuntimeError("test subject mismatch")
    return value, dev, tester


def write(path, value):
    data = json.dumps(value, sort_keys=True).encode()
    if len(data) > LIMIT:
        raise RuntimeError("fixture record too large")
    staging = Path(str(path) + "." + uuid.uuid4().hex + ".partial")
    fd = os.open(staging, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data); handle.flush()
            os.fchmod(handle.fileno(), 0o644); os.fsync(handle.fileno())
        os.link(staging, path, follow_symlinks=False)
    finally:
        staging.unlink()


def wait_record(path, uid, run, boot):
    deadline = time.monotonic() + 120
    while not path.exists() or path.lstat().st_nlink == 2:
        if time.monotonic() >= deadline:
            raise RuntimeError("peer coordination timed out")
        time.sleep(0.05)
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as handle:
        info = os.fstat(handle.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_uid != uid or info.st_nlink != 1 or stat.S_IMODE(info.st_mode) != 0o644 or info.st_size > LIMIT:
            raise RuntimeError("unsafe peer fixture record")
        data = handle.read(LIMIT + 1)
    if len(data) != info.st_size:
        raise RuntimeError("peer record changed")
    value = json.loads(data)
    if value.get("run_id") != run or value.get("boot_id") != boot or value.get("uid") != uid:
        raise RuntimeError("peer fixture binding mismatch")
    return value


def connection(path, uid, pid):
    stream = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    stream.settimeout(5)
    stream.connect(str(path))
    actual_pid, actual_uid, _ = struct.unpack("3i", stream.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i")))
    if actual_uid != uid or actual_pid != pid:
        stream.close()
        raise RuntimeError("actual server credentials mismatch")
    return stream


def exact(stream, count):
    data = bytearray()
    while len(data) < count:
        chunk = stream.recv(count - len(data))
        if not chunk:
            raise EOFError("authenticated server closed stream")
        data.extend(chunk)
    return bytes(data)


def call(stream, operation):
    correlation = str(uuid.uuid4())
    data = json.dumps({"schema_version":1, "request_id":correlation, "operation":operation}).encode()
    if len(data) > LIMIT:
        raise RuntimeError("test request exceeds task limit")
    stream.sendall(struct.pack(">I", len(data)) + data)
    length, = struct.unpack(">I", exact(stream, 4))
    if not 1 <= length <= 1024**2:
        raise RuntimeError("reply frame exceeds limit")
    value = json.loads(exact(stream, length))
    if value["schema_version"] != 1 or value["request_id"] != correlation or value["operation"] != "response":
        raise RuntimeError("reply correlation mismatch")
    return value


def daemon(directory):
    directory.mkdir(mode=0o700)
    if directory.resolve() != directory:
        raise RuntimeError("fixture directory contains a symlink")
    endpoint = directory / "session.sock"
    process = subprocess.Popen(["/run/current-system/sw/bin/aios-sessiond", "--socket", str(endpoint)],
                               stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    deadline = time.monotonic() + 5
    while not endpoint.exists():
        if process.poll() is not None or time.monotonic() >= deadline:
            process.kill(); process.wait()
            raise RuntimeError("isolated real daemon not ready")
        time.sleep(0.01)
    if endpoint.stat().st_uid != os.geteuid() or stat.S_IMODE(endpoint.stat().st_mode) != 0o600:
        raise RuntimeError("real daemon socket permissions mismatch")
    return process, endpoint


def cleanup(process, directory, endpoint):
    if process.poll() is None:
        process.terminate()
    process.wait(timeout=5)
    info = endpoint.lstat()
    if not stat.S_ISSOCK(info.st_mode) or info.st_uid != os.geteuid() or directory.lstat().st_uid != os.geteuid():
        raise RuntimeError("fixture ownership changed; cleanup refused")
    endpoint.unlink(); directory.rmdir()


def main():
    if len(sys.argv) not in (3, 4) or sys.argv[1] not in ("owner", "peer"):
        raise RuntimeError("fixed owner/peer scenario required")
    role, run = sys.argv[1:3]
    if str(uuid.UUID(run)) != run or len(sys.argv) != (3 if role == "owner" else 4):
        raise RuntimeError("invalid qualification binding")
    identity, dev, tester = subject(role)
    base = Path("/tmp") / ("aios-ipc-" + run)
    if Path("/tmp").resolve() != Path("/tmp"):
        raise RuntimeError("unexpected temporary directory")
    boot = identity["boot_id"]
    common = {"schema_version":1, "run_id":run, "boot_id":boot, "uid":os.geteuid(), "pid":os.getpid()}
    paths = {name: Path(str(base) + "." + name + ".json") for name in ("offer", "protected", "exposed", "peer")}
    def publish(name, value):
        write(paths[name], {**common, **value})
    process = None
    try:
        if role == "owner":
            process, endpoint = daemon(base)
            with connection(endpoint, dev, process.pid) as first:
                response = call(first, {"kind":"submit", "request":{"mode":"ask", "text":"Synthetic private two-user qualification prompt", "client_nonce":run}})
                assert response["error"] is None
                task = response["data"]["request_id"]
                before = call(first, {"kind":"get_status", "task_id":task})
                publish("offer", {"server_pid":process.pid, "task_id":task})
                protected = wait_record(paths["protected"], tester, run, boot)
                assert protected["private_endpoint_denied"] is True
                # Expose ONLY this isolated fixture to test kernel credential
                # denial even if filesystem permissions were misconfigured.
                base.chmod(0o755); endpoint.chmod(0o666)
                publish("exposed", {"isolated_fixture_only":True})
                peer = wait_record(paths["peer"], tester, run, boot)
                assert peer["wrong_uid_reconnect_denied"] is True
                endpoint.chmod(0o600); base.chmod(0o700)
                after = call(first, {"kind":"get_status", "task_id":task})
                assert before["data"] == after["data"] and after["data"]["mutation_performed"] is False
                print(json.dumps({**common, "identity":identity, "evidence_kind":"real-two-guest-UIDs-isolated-socket",
                    "peer":peer, "original_owner_task_unchanged":True, "original_owner_still_authorized":True,
                    "filesystem_denial":protected, "test_only_socket_permissions_restored":True}), flush=True)
        else:
            offer = wait_record(paths["offer"], dev, run, boot)
            try:
                probe = connection(base / "session.sock", dev, offer["server_pid"])
            except PermissionError as error:
                if error.errno not in (errno.EACCES, errno.EPERM): raise
            else:
                probe.close(); raise RuntimeError("another UID accessed the private fixture")
            publish("protected", {"private_endpoint_denied":True})
            exposed = wait_record(paths["exposed"], dev, run, boot)
            assert exposed["isolated_fixture_only"] is True
            denied_operations = []
            for kind in ("get_status", "get_events", "cancel", "forget"):
                operation = {"kind":kind, "task_id":offer["task_id"]}
                if kind == "get_events": operation.update(after_sequence=0, limit=10)
                with connection(base / "session.sock", dev, offer["server_pid"]) as wrong_uid:
                    try:
                        call(wrong_uid, operation)
                    except (EOFError, ConnectionResetError, BrokenPipeError):
                        denied_operations.append(kind)
                    else:
                        raise RuntimeError("another actual UID received private task data or authority")
            # Explicit host-selected synthetic session, never a guessed one.
            selected = sys.argv[3]
            ui_directory = Path(str(base) + "-tester")
            process, endpoint = daemon(ui_directory)
            with connection(endpoint, tester, process.pid) as own:
                caps = call(own, {"kind":"get_capabilities"})["data"]
                assert caps["ui_enabled"] is False
                candidate = call(own, {"kind":"select_ui_session", "session_id":selected})
                assert candidate["error"] is None
                value = candidate["data"]
                assert value["session"]["uid"] == tester and value["session"]["id"] == selected
                assert value["session"]["kind"] == "wayland" and value["session"]["active"] is True and value["session"]["remote"] is False
                assert value["confirmation_required"] is True and value["ui_authorized"] is False
                denied = call(own, {"kind":"submit", "request":{"mode":"act", "text":"Synthetic unconfirmed UI request", "client_nonce":run,
                                                                    "selected_session_handle":value["candidate_handle"]}})
                assert denied["error"]["code"] == "AUTH_REQUIRED"
                # The tester owns a real session broker too, but cannot resolve
                # the first UID's task through its own authorized endpoint.
                absent = call(own, {"kind":"get_status", "task_id":offer["task_id"]})
                assert absent["error"]["code"] == "TARGET_NOT_FOUND"
                publish("peer", {"wrong_uid_reconnect_denied":True, "denied_operations":denied_operations,
                    "private_data_received":False, "own_daemon_foreign_task_denied":True,
                    "selected_live_graphical_candidate":value, "unconfirmed_ui_denial":denied["error"],
                    "ui_enabled":False, "identity":identity})
            cleanup(process, ui_directory, endpoint); process = None
    finally:
        if process is not None:
            cleanup(process, base if role == "owner" else Path(str(base) + "-tester"), endpoint)
        # Coordination records are intentionally public synthetic identifiers.
        # Keep them until the host collects the bounded qualification report.


if __name__ == "__main__":
    main()
