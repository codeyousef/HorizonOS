#!/usr/bin/env python3
"""Execute one candidate build from a sealed disposable-image request."""
import json
import os
from pathlib import Path
import socket
import stat
import struct
import uuid
import subprocess

MARKER = Path("/tmp/aios-builder-request")
REPORT = Path("/run/aios-builder-qualification/report.json")
SOCKET = Path("/run/aios-build/worker.sock")
CANDIDATES = Path("/var/lib/aios/candidates")
DIGEST = set("0123456789abcdef")
REQUEST_FIELDS = {
    "request_token", "schema_version", "operation", "request_id", "plan_id",
    "candidate_sha256", "template_sha256", "managed_sha256", "baseline_closure",
    "max_build_bytes", "max_download_bytes", "recovery_reserve_bytes", "approved_cache",
}


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()

def compact(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def target_identity():
    result = subprocess.run(
        ["/run/current-system/sw/bin/aios-guest-identity"],
        capture_output=True, timeout=10, check=True,
    )
    if len(result.stdout) > 65536:
        raise RuntimeError("target identity exceeds bound")
    return json.loads(result.stdout)



def valid_digest(value):
    return isinstance(value, str) and len(value) == 64 and set(value) <= DIGEST


def receive(connection, bound):
    length = struct.unpack(">I", connection.recv(4, socket.MSG_WAITALL))[0]
    if length == 0 or length > bound:
        raise RuntimeError("candidate builder reply length is invalid")
    return json.loads(connection.recv(length, socket.MSG_WAITALL))

def transact(request):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
        connection.settimeout(1260)
        connection.connect(str(SOCKET))
        peer = struct.unpack(
            "3i", connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12)
        )
        if peer[0] <= 0 or peer[1] == 0 or peer[2] == 0:
            raise RuntimeError("candidate builder peer identity is invalid")
        encoded = json.dumps(request, sort_keys=True, separators=(",", ":")).encode()
        connection.sendall(struct.pack(">I", len(encoded)) + encoded)
        return receive(connection, 8 * 1024 * 1024), peer


def main():
    identity = target_identity()
    token = None
    report = {"verified": False, "identity": identity}
    try:
        marker = MARKER.lstat()
        if not stat.S_ISREG(marker.st_mode) or marker.st_uid != 1000 or stat.S_IMODE(marker.st_mode) != 0o600 or marker.st_size > 16 * 1024:
            raise RuntimeError("candidate builder request identity is invalid")
        request = json.loads(MARKER.read_bytes())
        if set(request) != REQUEST_FIELDS or canonical(request) != MARKER.read_bytes():
            raise RuntimeError("candidate builder request is not canonical or has unknown fields")
        token = request.pop("request_token")
        if not isinstance(token, str) or len(token) != 32 or set(token) > DIGEST:
            raise RuntimeError("candidate builder request token is invalid")
        if request.get("schema_version") != 1 or request.get("operation") != "build":
            raise RuntimeError("candidate builder operation is invalid")
        for key in ("request_id", "plan_id"):
            if str(uuid.UUID(request[key])) != request[key]:
                raise RuntimeError(f"candidate builder {key} is invalid")
        for key in ("candidate_sha256", "template_sha256", "managed_sha256"):
            if not valid_digest(request.get(key)):
                raise RuntimeError(f"candidate builder {key} is invalid")
        candidate = CANDIDATES / request["candidate_sha256"]
        manifest_path = candidate / "candidate.json"
        candidate_state = candidate.lstat()
        manifest_state = manifest_path.lstat()
        manifest = json.loads(manifest_path.read_bytes())
        if (not stat.S_ISDIR(candidate_state.st_mode) or candidate_state.st_uid != 0
                or stat.S_IMODE(candidate_state.st_mode) != 0o555
                or not stat.S_ISREG(manifest_state.st_mode) or manifest_state.st_uid != 0
                or stat.S_IMODE(manifest_state.st_mode) != 0o444
                or compact(manifest) != manifest_path.read_bytes()
                or manifest.get("template_sha256") != request["template_sha256"]
                or manifest.get("managed_sha256") != request["managed_sha256"]):
            raise RuntimeError("sealed candidate metadata does not match request")
        denials = {}
        probes = (
            ("substituter", {"approved_cache": "https://unapproved.invalid"}, "Invalid"),
            ("template", {"template_sha256": "0" * 64}, "Candidate"),
            ("baseline", {"baseline_closure": os.path.realpath("/run/current-system/sw")}, "Baseline"),
            ("resource", {"max_build_bytes": 16 * 1024 * 1024 * 1024 + 1}, "Invalid"),
        )
        for name, changes, expected_error in probes:
            probe = request | changes | {
                "request_id": str(uuid.uuid4()),
                "plan_id": str(uuid.uuid4()),
            }
            denied, _ = transact(probe)
            if denied.get("ok") is not False or denied.get("error") != expected_error:
                raise RuntimeError(
                    f"candidate builder accepted {name} probe: "
                    + json.dumps(denied, sort_keys=True)
                )
            denials[name] = denied["error"]
        response, peer = transact(request)
        peer_pid, peer_uid, peer_gid = peer
        data = response.get("data", {})
        if response.get("ok") is not True or data.get("schema_version") != 1 or data.get("request_id") != request["request_id"]:
            raise RuntimeError("candidate builder rejected fixed qualification request: " + json.dumps(response, sort_keys=True))
        build = data.get("build", {})
        roots = {
            "candidate": (Path(build.get("gc_root", "")), build.get("closure")),
            "prior": (Path(build.get("prior_gc_root", "")), request["baseline_closure"]),
        }
        for name, (root, expected) in roots.items():
            state = root.lstat()
            if (not stat.S_ISLNK(state.st_mode) or state.st_uid != peer_uid
                    or os.path.realpath(root) != expected):
                raise RuntimeError(f"candidate builder {name} GC root is invalid")
        report.update({"verified": True, "request_token": token, "request": request,
                       "response": response, "denials": denials,
                       "builder_peer": {"pid": peer_pid, "uid": peer_uid, "gid": peer_gid},
                       "current_system_after": os.path.realpath("/run/current-system")})
        if target_identity() != identity:
            raise RuntimeError("target changed during candidate build")
    except Exception as error:
        report.update({"request_token": token, "error": {"type": type(error).__name__, "message": str(error)}})
    finally:
        try:
            MARKER.unlink()
        except FileNotFoundError:
            pass
        REPORT.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
        temporary = REPORT.with_suffix(".tmp")
        temporary.write_bytes(canonical(report))
        os.chmod(temporary, 0o644)
        os.replace(temporary, REPORT)
    if not report["verified"]:
        raise RuntimeError(report["error"]["message"])


if __name__ == "__main__":
    main()
