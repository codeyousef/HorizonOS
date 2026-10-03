#!/usr/bin/env python3
"""Build an enrolled development OS from a frozen, pure candidate.

This is the development job route, not the production aios-buildd authority.
No activation, profile write, bootloader installation or private-key read occurs.
"""
import base64
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import time

import snapshot as source
import dev_deploy

ENROLLMENT = "nix/machines/aios-dev/enrollment.json"
RESERVE_BYTES = 8 * 1024**3
LOCKED = ["--no-update-lock-file", "--no-write-lock-file"]


def build_arguments(working, output_link):
    return ["nix", "build", "--json", "--out-link", str(output_link), *LOCKED,
            "--option", "pure-eval", "true", "--option", "allow-import-from-derivation", "false",
            "--option", "substituters", "https://cache.nixos.org",
            "path:" + str(working) + "#nixosConfigurations.aios-dev.config.system.build.toplevel"]


def public_key(text):
    lines = [line.strip() for line in text.splitlines() if line.strip() and not line.startswith("#")]
    if len(lines) != 1:
        raise ValueError("exactly one enrolled developer key is required")
    parts = lines[0].split()
    if len(parts) < 2 or parts[0] != "ssh-ed25519":
        raise ValueError("only the enrolled Ed25519 key is supported")
    wire = base64.b64decode(parts[1], validate=True)
    if len(wire) != 51 or wire[:19] != b"\x00\x00\x00\x0bssh-ed25519\x00\x00\x00\x20":
        raise ValueError("invalid Ed25519 key encoding")
    return " ".join(parts[:2])


def read_enrolled_key():
    installed = Path("/etc/ssh/authorized_keys.d/dev")
    for parent in (Path("/etc"), Path("/etc/ssh"), installed.parent):
        info = parent.stat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022:
            raise ValueError("public enrollment parent is not administrator-owned")
    if installed.lstat().st_uid != 0:
        raise ValueError("developer enrollment is not root-owned")
    target = installed.resolve(strict=True)
    # NixOS may publish authorized_keys as an administrator-owned mutable copy.
    # Capture stable bytes now, freeze them in the candidate, and recheck after
    # building. Evaluation never reads the live key file.
    if not target.is_relative_to("/nix/store") and target != installed:
        raise ValueError("developer enrollment is outside its installed scope")
    descriptor = os.open(target, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as handle:
        info = os.fstat(handle.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022 or info.st_size > 4096:
            raise ValueError("unsafe installed public key")
        data = handle.read(4097)
        after = os.fstat(handle.fileno())
        if len(data) != info.st_size or (after.st_size, after.st_mtime_ns, after.st_ctime_ns) != (info.st_size, info.st_mtime_ns, info.st_ctime_ns):
            raise ValueError("public enrollment changed during capture")
        return public_key(data.decode("ascii"))


def enrollment(identity, key):
    dev_deploy.validate_identity(identity)
    if identity["os_id"] != "nixos" or identity["guest_role"] != "development" or identity["disk_serial"] != "AIOS_DEV_ROOT" or identity["management_channel"] != "ssh-development":
        raise ValueError("system build requires the enrolled development target")
    return {"schema_version": 1, "dmi_uuid": identity["dmi_uuid"],
            "installation_uuid": identity["installation_uuid"], "guest_role": "development",
            "disk_serial": "AIOS_DEV_ROOT", "management_channel": "ssh-development",
            "authorized_key": public_key(key)}


def prepare(release, destination, manifest, enrolled):
    source.verify_tree(release, manifest, published=True)
    if ENROLLMENT in {item["path"] for item in manifest["files"]}:
        raise ValueError("source cannot override enrolled machine identity")
    destination.mkdir(mode=0o700)
    files = []
    try:
        for item in manifest["files"]:
            data, mode = source.read_regular(release, item["path"], published=True)
            if mode != item["mode"] or len(data) != item["size"] or hashlib.sha256(data).hexdigest() != item["sha256"]:
                raise ValueError("source changed during candidate creation")
            path = destination / item["path"]
            path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
            path.write_bytes(data)
            path.chmod(mode)
            files.append(item)
        data = source.canonical(enrolled)
        path = destination / ENROLLMENT
        path.write_bytes(data)
        path.chmod(0o644)
        files.append({"path": ENROLLMENT, "mode": 0o644, "size": len(data), "sha256": hashlib.sha256(data).hexdigest()})
        candidate = {**manifest, "dirty": True, "files": sorted(files, key=lambda item: item["path"])}
        digest = source.validate_manifest(candidate)
        (destination / source.MANIFEST).write_bytes(source.canonical(candidate))
        (destination / source.MANIFEST).chmod(0o644)
        for parent, _, _ in os.walk(destination):
            Path(parent).chmod(0o700)
        source.verify_tree(destination, candidate)
        for parent, _, names in os.walk(destination):
            for name in names:
                path = Path(parent) / name
                path.chmod(0o555 if path.stat().st_mode & 0o111 else 0o444)
            Path(parent).chmod(0o555)
        source.verify_tree(destination, candidate, published=True)
        return candidate, digest
    except BaseException:
        for parent, _, names in os.walk(destination):
            Path(parent).chmod(0o700)
            for name in names:
                (Path(parent) / name).chmod(0o600)
        shutil.rmtree(destination)
        raise


def store_path(path):
    value = str(path)
    if not dev_deploy.STORE.fullmatch(value):
        raise ValueError("invalid realized store root")
    return value


def pointers():
    return {"running": store_path(Path("/run/current-system").resolve(strict=True)),
            "profile": store_path(Path("/nix/var/nix/profiles/system").resolve(strict=True)),
            "booted": store_path(Path("/run/booted-system").resolve(strict=True))}


def main():
    if len(sys.argv) != 2:
        raise ValueError("registered job directory required")
    import jobs
    directory = Path(sys.argv[1])
    if directory.parent != jobs.job_root() or jobs.job_id(directory.name) != directory.name:
        raise ValueError("unregistered system build directory")
    record = jobs.read_record(directory)
    request = record["request"]
    if request["kind"] != "build-system" or source.identity() != request["identity"]:
        raise ValueError("system build target changed")
    release = Path(__file__).resolve().parents[2]
    manifest = source.decode((release / source.MANIFEST).read_bytes())
    if source.validate_manifest(manifest) != request["snapshot_digest"]:
        raise ValueError("system build source differs")
    space = os.statvfs("/nix/store")
    available = space.f_bavail * space.f_frsize
    if available < RESERVE_BYTES:
        raise ValueError("system build recovery reserve unavailable")
    expected_locks = {name: hashlib.sha256((release / name).read_bytes()).hexdigest() for name in ("flake.lock", "Cargo.lock")}
    previous = pointers()
    enrolled = enrollment(request["identity"], read_enrolled_key())
    working = directory / "system-candidate"
    candidate, digest = prepare(release, working, manifest, enrolled)
    report = {"schema_version": 1, "evidence_kind": "real-development-system-build", "identity": request["identity"],
              "source_digest": request["snapshot_digest"], "candidate_digest": digest, "candidate_manifest": candidate,
              "enrollment_sha256": hashlib.sha256(source.canonical(enrolled)).hexdigest(),
              "lock_hashes": expected_locks, "prior_pointers": previous, "available_store_bytes_before": available, "recovery_reserve_bytes": RESERVE_BYTES,
              "commands": [], "activation_performed": False, "product_builder_authority_verified": False,
              "limitations": ["This development job is not the root broker or product aios-buildd.",
                              "GC roots and source copies belong to dev; production root retention remains required.",
                              "Boot-selected metadata is not readable here; booted-system is reported separately.",
                              "The space reserve is a precheck; hard build/download quotas remain unimplemented.",
                              "No boot, Plasma login, model-service activation or guarded rollback is verified."]}
    jobs.atomic(directory / "system-build.json", report)

    def run(argv, *, structured=False):
        if source.identity() != request["identity"]:
            raise ValueError("target changed before development build operation")
        started = time.monotonic()
        environment = {name: os.environ[name] for name in ("HOME", "PATH", "TMPDIR", "XDG_CACHE_HOME")}
        environment.update(LANG="C.UTF-8", NIX_USER_CONF_FILES="/dev/null", NIX_REMOTE="daemon")
        result = subprocess.run(argv, capture_output=True, timeout=1500, check=False, env=environment)
        report["commands"].append({"argv": argv, "upstream_exit": result.returncode,
                                   "elapsed_seconds": round(time.monotonic() - started, 3)})
        jobs.atomic(directory / "system-build.json", report)
        sys.stderr.buffer.write(result.stderr)
        if result.returncode:
            sys.stdout.buffer.write(result.stdout)
            raise RuntimeError("registered system build command failed")
        return source.decode(result.stdout) if structured else result.stdout.decode().strip()

    try:
        for name, path in previous.items():
            run(["nix-store", "--add-root", str(directory / ("prior-" + name)), "--indirect", "--realise", path])
        # An out-link roots the result for this development job's lifetime.
        outputs = run(build_arguments(working, directory / "candidate-root"), structured=True)
        if len(outputs) != 1 or set(outputs[0]["outputs"]) != {"out"}:
            raise ValueError("unexpected system output count")
        realized = store_path(outputs[0]["outputs"]["out"])
        for name, expected in (("installation-uuid", enrolled["installation_uuid"]), ("expected-dmi-uuid", enrolled["dmi_uuid"]), ("guest-role", "development")):
            if (Path(realized) / "etc/aios" / name).read_text().strip() != expected:
                raise ValueError("built system enrollment differs")
        before = run(["nix", "path-info", "--json", "--recursive", previous["running"]], structured=True)
        after = run(["nix", "path-info", "--json", "--recursive", realized], structured=True)
        report.update(built_output=realized, output_root=str(directory / "candidate-root"),
                      closure_diff={"added_paths": sorted(set(after)-set(before)), "removed_paths": sorted(set(before)-set(after)),
                                    "prior_nar_bytes": sum(v["narSize"] for v in before.values()),
                                    "candidate_nar_bytes": sum(v["narSize"] for v in after.values())})
        for name, expected in expected_locks.items():
            if hashlib.sha256((working / name).read_bytes()).hexdigest() != expected:
                raise ValueError("build changed source locks")
        source.verify_tree(working, candidate, published=True)
        if pointers() != previous or source.identity() != request["identity"] or read_enrolled_key() != enrolled["authorized_key"]:
            raise ValueError("build changed running system identity/pointers")
        space_after = os.statvfs("/nix/store")
        report["available_store_bytes_after"] = space_after.f_bavail * space_after.f_frsize
        if report["available_store_bytes_after"] < RESERVE_BYTES:
            raise ValueError("build consumed the development recovery reserve")
        report["state"] = "succeeded"
    except BaseException as error:
        report.update(state="failed", failure=type(error).__name__, pointers_after=pointers())
        jobs.atomic(directory / "system-build.json", report)
        print("AIOS_SYSTEM_BUILD " + json.dumps(report, sort_keys=True), flush=True)
        raise
    jobs.atomic(directory / "system-build.json", report)
    print("AIOS_SYSTEM_BUILD " + json.dumps(report, sort_keys=True), flush=True)


if __name__ == "__main__":
    main()
