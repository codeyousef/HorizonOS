#!/usr/bin/env python3
"""Verify one sealed candidate build without activation in the disposable image."""
import hashlib
import json
import os
from pathlib import Path
import secrets
import stat
import subprocess
import time
import uuid

import snapshot

MARKER = Path("/tmp/aios-builder-request")
REPORT = Path("/run/aios-builder-qualification/report.json")
CANDIDATES = Path("/var/lib/aios/candidates")
DIGEST = set("0123456789abcdef")


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()

def compact(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def unit_state(name):
    result = subprocess.run(
        ["systemctl", "show", name, "--property=ActiveState", "--property=SubState",
         "--property=Result", "--property=ExecMainStatus"],
        capture_output=True, text=True, timeout=10, check=False,
    )
    return {"exit_status": result.returncode, "output": result.stdout.strip()}

def unit_journal():
    result = subprocess.run(
        ["journalctl", "--unit=aios-builder-qualification.service", "--no-pager",
         "--lines=20", "--output=short-monotonic"],
        capture_output=True, text=True, timeout=10, check=False,
    )
    return {"exit_status": result.returncode, "output": result.stdout[-16 * 1024:].strip()}




def candidate():
    choices = []
    for path in CANDIDATES.iterdir():
        if len(path.name) != 64 or set(path.name) > DIGEST:
            continue
        state = path.lstat()
        manifest_path = path / "candidate.json"
        manifest_state = manifest_path.lstat()
        data = manifest_path.read_bytes()
        manifest = json.loads(data)
        if (not stat.S_ISDIR(state.st_mode) or state.st_uid != 0 or stat.S_IMODE(state.st_mode) != 0o555
                or not stat.S_ISREG(manifest_state.st_mode) or manifest_state.st_uid != 0
                or stat.S_IMODE(manifest_state.st_mode) != 0o444
                or compact(manifest) != data or hashlib.sha256(data).hexdigest() != path.name):
            raise RuntimeError("sealed candidate metadata is invalid")
        choices.append((path.name, manifest))
    if len(choices) != 1:
        raise RuntimeError(f"expected one Executor1-prepared candidate, observed {len(choices)}")
    return choices[0]


def main():
    identity = snapshot.identity()
    initial_path = unit_state("aios-builder-qualification.path")
    if "ActiveState=active" not in initial_path["output"]:
        raise RuntimeError("candidate builder qualification path is unavailable: "
                           + json.dumps({"path": initial_path, "journal": unit_journal()}, sort_keys=True))
    before = os.path.realpath("/run/current-system")
    candidate_sha256, manifest = candidate()
    token = secrets.token_hex(16)
    request = {
        "request_token": token,
        "schema_version": 1,
        "operation": "build",
        "request_id": str(uuid.uuid4()),
        "plan_id": str(uuid.uuid4()),
        "candidate_sha256": candidate_sha256,
        "template_sha256": manifest["template_sha256"],
        "managed_sha256": manifest["managed_sha256"],
        "baseline_closure": before,
        "max_build_bytes": 16 * 1024 * 1024 * 1024,
        "max_download_bytes": 4 * 1024 * 1024 * 1024,
        "recovery_reserve_bytes": 8 * 1024 * 1024 * 1024,
        "approved_cache": "https://cache.nixos.org",
    }
    try:
        descriptor = os.open(MARKER, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(descriptor, "wb") as marker:
            marker.write(canonical(request))
        state = MARKER.lstat()
        if state.st_uid != os.geteuid() or stat.S_IMODE(state.st_mode) != 0o600:
            raise RuntimeError("candidate builder marker identity changed")
        deadline = time.monotonic() + 1320
        report = None
        while time.monotonic() < deadline:
            if REPORT.exists():
                observed = json.loads(REPORT.read_bytes())
                if observed.get("request_token") == token:
                    report = observed
                    break
            time.sleep(0.5)
        if report is None:
            raise RuntimeError("fixed candidate builder fixture did not publish a fresh result: "
                               + json.dumps({"path": unit_state("aios-builder-qualification.path"),
                                             "service": unit_state("aios-builder-qualification.service")},
                                            sort_keys=True))
        if report.get("verified") is not True:
            raise RuntimeError("candidate builder fixture failed: " + json.dumps(report.get("error"), sort_keys=True))
        if report.get("identity") != identity or snapshot.identity() != identity:
            raise RuntimeError("candidate build result is not bound to the enrolled target")
        if report.get("request") != {key: value for key, value in request.items() if key != "request_token"}:
            raise RuntimeError("candidate builder executed a different request")
        if report.get("current_system_after") != before or os.path.realpath("/run/current-system") != before:
            raise RuntimeError("candidate build changed the active system")
        response = report.get("response", {})
        data = response.get("data", {})
        build = data.get("build", {})
        if (response.get("ok") is not True or data.get("request_id") != request["request_id"]
                or build.get("candidate_sha256") != candidate_sha256
                or not str(build.get("derivation", "")).startswith("/nix/store/")
                or not str(build.get("derivation", "")).endswith(".drv")
                or not str(build.get("closure", "")).startswith("/nix/store/")
                or build.get("gc_root") != "/var/lib/aios/build/roots/" + request["plan_id"] + "-candidate"
                or build.get("prior_gc_root") != "/var/lib/aios/build/roots/" + request["plan_id"] + "-prior"
                or not isinstance(build.get("nar_bytes"), int) or build["nar_bytes"] <= 0
                or build["nar_bytes"] > request["max_build_bytes"]
                or build.get("measured_download_bytes") != 0
                or len(str(build.get("inventory_sha256", ""))) != 64
                or not isinstance(build.get("added_paths"), list)
                or not isinstance(build.get("removed_paths"), list)
                or report.get("denials") != {
                    "baseline": "Baseline",
                    "resource": "Invalid",
                    "substituter": "Invalid",
                    "template": "Candidate",
                }):
            raise RuntimeError("candidate build evidence is incomplete: " + json.dumps(response, sort_keys=True))
        peer = report.get("builder_peer", {})
        if peer.get("uid") in (None, 0) or peer.get("gid") in (None, 0) or peer.get("pid", 0) <= 0:
            raise RuntimeError("candidate builder peer evidence is incomplete")
        print("AIOS_INSTALLED_BUILDER " + json.dumps(report, sort_keys=True), flush=True)
    finally:
        try:
            state = MARKER.lstat()
            if stat.S_ISREG(state.st_mode) and state.st_uid == os.geteuid():
                MARKER.unlink()
        except FileNotFoundError:
            pass


if __name__ == "__main__":
    main()
