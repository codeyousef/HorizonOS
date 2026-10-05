#!/usr/bin/env python3
"""Registered development jobs with durable records and scoped cancellation."""
import base64
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import shutil
import signal
import subprocess
import sys
import time
import uuid

import snapshot as source

PACKAGES = ("aios-consent-ui", "aios-core", "aios-model", "aios-desktop", "aios-cli", "aios-dev-tools")
KINDS = {"resolve-lock", "build-packages", "build-system", "build-desktop-test", "test-unit", "system-info-smoke", "service-inspection-smoke", "journal-inspection-smoke", "public-session-smoke", "installed-session-inference-smoke", "consent-ui-smoke", "accessibility-smoke", "ui-provider-smoke", "model-compatibility-smoke", "upstream-compatibility-smoke", "host-boundary-smoke", "protocol-conformance-smoke", "model-profile-low-smoke", "model-profile-high-smoke", "model-inference-smoke", "model-service-smoke", "session-inference-smoke", "development-boundary-smoke", "guard-state-smoke", "managed-state-smoke", "broker-preparation-smoke", "installed-runtime-smoke", "installed-policy-smoke", "installed-development-smoke", "installed-guard-smoke", "installed-model-smoke", "installed-model-idle-smoke", "installed-model-lifecycle-smoke", "supervision-probe"}
TERMINAL = {"succeeded", "failed", "cancelled", "interrupted"}
LIMIT = 4 * 1024**2
cancelled = False


def stamp():
    return datetime.now(timezone.utc).isoformat()


def sanitize(text):
    text = re.sub(r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----.*?(?:-----END [A-Z0-9 ]*PRIVATE KEY-----|$)", "[REDACTED PRIVATE KEY]", text, flags=re.S)
    text = re.sub(r"(?im)(authorization\s*[:=]\s*)[^\n]*", r"\1[REDACTED]", text)
    text = re.sub(r"(?i)((?:password|authorization|api[_-]?key|access[_-]?token|secret)\s*[:=]\s*)[^\s,;]+", r"\1[REDACTED]", text)
    text = re.sub(r"\bsk-[A-Za-z0-9_-]{16,}\b", "[REDACTED TOKEN]", text)
    text = re.sub(r"\x1b\[[0-9;?]*[A-Za-z]", "", text)
    return "".join(c for c in text if c in "\n\t" or 32 <= ord(c) != 127)


def atomic(path, value):
    temporary = path.parent / (".record-" + str(uuid.uuid4()))
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(source.canonical(value))
            handle.flush()
            os.fsync(handle.fileno())
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def job_root():
    # Same home/ownership/symlink constraints as source publication.
    import pwd
    home = Path(pwd.getpwuid(os.getuid()).pw_dir)
    return source.release_root(str(home / ".aios-jobs"))


def job_id(value):
    if not isinstance(value, str) or str(uuid.UUID(value)) != value:
        raise ValueError("invalid job ID")
    return value


def read_record(directory):
    info = directory.lstat()
    if not directory.is_dir() or directory.is_symlink() or info.st_uid != os.getuid() or info.st_mode & 0o777 != 0o700:
        raise ValueError("unsafe job directory")
    path = directory / "report.json"
    info = path.lstat()
    if path.is_symlink() or not path.is_file() or info.st_uid != os.getuid() or info.st_nlink != 1 or info.st_mode & 0o777 != 0o600 or info.st_size > 1024**2:
        raise ValueError("unsafe job record")
    return source.decode(path.read_bytes())


def ticks(pid):
    text = Path(f"/proc/{pid}/stat").read_text()
    fields = text[text.rfind(")") + 2:].split()
    if fields[0] == "Z":
        raise ValueError("worker is a zombie")
    return int(fields[19])


def process_matches(record):
    pid = record.get("worker_pid")
    if type(pid) is not int or pid <= 1:
        return False
    try:
        return Path(f"/proc/{pid}").stat().st_uid == os.getuid() and ticks(pid) == record["worker_start_ticks"]
    except (OSError, KeyError, ValueError):
        return False


def commands(kind, release, package=None, job_directory=None):
    reference = "path:" + str(release)
    locked = ["--no-update-lock-file", "--no-write-lock-file"]
    if kind == "build-packages":
        names = [package] if package else list(PACKAGES)
        if any(name not in PACKAGES for name in names):
            raise ValueError("unregistered package")
        return [["nix", "build", "--json", "--no-link", *locked, *[reference + "#" + name for name in names]]]
    if kind in {"build-system", "build-desktop-test"}:
        if job_directory is None:
            raise ValueError("system build requires its registered job directory")
        return [["python3", str(release / "tools/guest/build_system.py"), str(job_directory)]]
    if kind == "test-unit":
        return [["nix", "build", "--json", "--no-link", *locked, reference + "#checks.x86_64-linux.consent-ui"],
                ["nix", "develop", *locked, reference, "--command", "cargo", "test", "--locked", "--workspace"],
                ["python3", "-m", "unittest", "discover", "-s", "tests/unit", "-q"]]
    if kind == "resolve-lock":
        # A workspace dependency change must not refresh unrelated locked
        # registry packages. Initial resolution still creates a missing lock.
        cargo = ["cargo", "update", "--workspace"] if (release / "Cargo.lock").exists() else ["cargo", "generate-lockfile"]
        return [["nix", "flake", "lock", reference], ["nix", "develop", *locked, reference + "#lock-resolution", "--command", *cargo]]
    if kind == "system-info-smoke":
        return [["nix", "develop", *locked, reference, "--command", "cargo", "test", "--locked", "-p", "aios-cli", "--test", "system_info", "--", "--nocapture"]]
    if kind == "service-inspection-smoke":
        return [["nix", "develop", *locked, reference, "--command", "cargo", "test", "--locked", "-p", package_name, "--test", test_name, "--", "--nocapture"]
                for package_name, test_name in (("aios-cli", "service_inspect"), ("aios-session", "ipc"))] + [
                    ["python3", str(release / "tools/guest/service_inspection_smoke.py")]]
    if kind == "journal-inspection-smoke":
        return [["nix", "develop", *locked, reference, "--command", "cargo", "test", "--locked", "-p", "aios-exec", "--test", "journal_observer", "--", "--ignored", "--nocapture"]]
    if kind == "public-session-smoke":
        return [["nix", "develop", *locked, reference, "--command", "cargo", "test", "--locked", "-p", "aios-session", "--test", "dbus", "--", "--nocapture"],
                ["python3", str(release / "tools/guest/public_session_smoke.py")]]
    if kind == "consent-ui-smoke":
        return [["python3", str(release / "tools/guest/consent_ui_smoke.py")]]
    if kind == "installed-session-inference-smoke":
        return [["python3", str(release / "tools/guest/public_session_smoke.py"), "--installed-model"]]
    if kind == "ui-provider-smoke":
        return [["python3",str(release / "tools/guest/ui_provider_smoke.py")]]
    if kind == "accessibility-smoke":
        return [["nix","develop",*locked,reference,"--command","cargo","test","--locked","-p","aios-session","--test","accessibility_native","--","--ignored","--nocapture"]]
    if kind == "model-compatibility-smoke":
        return [["nix", "develop", *locked, reference + "#model-conversion", "--command", "python3", str(release / "tools/guest/model_conversion.py")]]
    if kind == "upstream-compatibility-smoke":
        return [["nix", "develop", *locked, reference + "#model-conversion", "--command", "python3", str(release / "tools/guest/upstream_compatibility.py")]]
    if kind in {"model-profile-low-smoke", "model-profile-high-smoke"}:
        profile = "low" if kind == "model-profile-low-smoke" else "high"
        return [["nix", "develop", *locked, reference + "#model-conversion", "--command", "python3", str(release / "tools/guest/model_profiles_smoke.py"), profile]]
    if kind == "protocol-conformance-smoke":
        return [["nix", "develop", *locked, reference, "--command", "cargo", "test", "--locked", "-p", package_name, "--test", test_name, "--", "--nocapture"]
                for package_name, test_name in (("aios-protocol", "conformance"), ("aios-session", "ipc"))]
    if kind == "host-boundary-smoke":
        return [["python3", str(release / "tools/guest/host_boundary_smoke.py")]]
    if kind == "model-inference-smoke":
        return [["python3",str(release / "tools/guest/model_inference_smoke.py")]]
    if kind == "broker-preparation-smoke":
        return [["python3", str(release / "tools/guest/broker_preparation_smoke.py")]]
    if kind == "installed-policy-smoke":
        return [["python3", str(release / "tools/guest/installed_policy_smoke.py")]]
    if kind == "installed-development-smoke":
        return [["python3", str(release / "tools/guest/installed_development_smoke.py")]]
    if kind == "installed-model-lifecycle-smoke":
        return [["python3", str(release / "tools/guest/installed_model_lifecycle_smoke.py")]]
    if kind == "installed-model-idle-smoke":
        return [["python3", str(release / "tools/guest/installed_model_idle.py")]]
    if kind == "installed-model-smoke":
        return [["python3", str(release / "tools/guest/installed_model_smoke.py")]]
    if kind == "installed-guard-smoke":
        return [["python3", str(release / "tools/guest/installed_guard_smoke.py")]]
    if kind == "installed-runtime-smoke":
        return [["python3", str(release / "tools/guest/installed_runtime_smoke.py")]]
    if kind == "managed-state-smoke":
        return [["python3", str(release / "tools/guest/managed_state_smoke.py")]]
    if kind == "guard-state-smoke":
        return [["python3", str(release / "tools/guest/guard_state_smoke.py")]]
    if kind == "development-boundary-smoke":
        return [["python3", str(release / "tools/guest/development_boundary_smoke.py")]]
    if kind == "session-inference-smoke":
        return [["python3",str(release / "tools/guest/session_inference_smoke.py")]]
    if kind == "model-service-smoke":
        return [["python3",str(release / "tools/guest/model_service_smoke.py")]]
    if kind == "supervision-probe":
        return [["python3", "-c", "import time; time.sleep(30)"]]
    raise ValueError("unregistered job kind")


def validate_start(request):
    if request["kind"] not in KINDS or request["package"] is not None and (request["kind"] != "build-packages" or request["package"] not in PACKAGES):
        raise ValueError("invalid registered job")
    digest = request["snapshot_digest"]
    if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise ValueError("invalid source digest")
    root = source.release_root(request["source_root"])
    release = root / digest
    metadata, _ = source.read_regular(release, source.MANIFEST, published=True)
    manifest = source.decode(metadata)
    if source.validate_manifest(manifest) != digest:
        raise ValueError("source manifest digest mismatch")
    source.verify_tree(release, manifest, published=True)
    paths = {item["path"] for item in manifest["files"]}
    if request["kind"] != "resolve-lock" and not {"flake.lock", "Cargo.lock"}.issubset(paths):
        return None, manifest
    return release, manifest


def invoke(arguments, cwd, environment, timeout=1800):
    global cancelled
    out, err = bytearray(), bytearray()
    child = subprocess.Popen(arguments, cwd=cwd, env=environment, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
    started = time.monotonic()
    reason = None
    try:
        with selectors.DefaultSelector() as selector:
            selector.register(child.stdout, selectors.EVENT_READ, out)
            selector.register(child.stderr, selectors.EVENT_READ, err)
            while selector.get_map():
                if cancelled or time.monotonic() - started > timeout:
                    reason = "cancelled" if cancelled else "timeout"
                    break
                for event, _ in selector.select(0.25):
                    chunk = os.read(event.fd, 65536)
                    if not chunk:
                        selector.unregister(event.fileobj)
                    else:
                        event.data.extend(chunk)
                        if len(out) + len(err) > LIMIT:
                            reason = "output-limit"
                            break
                if reason:
                    break
        if reason and child.poll() is None:
            # This process group belongs to the still-live child of this job.
            os.killpg(child.pid, signal.SIGTERM)
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
        status = child.wait(timeout=5)
        return status, bytes(out[:LIMIT]), bytes(err[:LIMIT]), reason
    finally:
        if child.poll() is None:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait()
        child.stdout.close()
        child.stderr.close()


def worker(directory):
    global cancelled
    def cancel(_signum, _frame):
        global cancelled
        cancelled = True
    signal.signal(signal.SIGTERM, cancel)
    report = read_record(directory)
    report.update(state="running", worker_pid=os.getpid(), worker_start_ticks=ticks(os.getpid()), started_at=stamp())
    atomic(directory / "report.json", report)
    code = 6
    try:
        request = report["request"]
        if source.identity() != request["identity"]:
            raise ValueError("job target changed")
        release, manifest = validate_start(request)
        if release is None:
            report["failure"] = "SOURCE_LOCKS_REQUIRED"
            code = 3
        else:
            working = release
            if request["kind"] == "resolve-lock":
                working = directory / "lock-source"
                shutil.copytree(release, working)
                for parent, _, names in os.walk(working):
                    Path(parent).chmod(0o700)
                    for name in names:
                        path = Path(parent) / name
                        path.chmod(0o755 if path.stat().st_mode & 0o111 else 0o644)
            cache = directory / "cache"
            cache.mkdir(mode=0o700)
            environment = {"HOME": os.environ["HOME"], "PATH": "/run/current-system/sw/bin", "LANG": "C.UTF-8",
                           "XDG_CACHE_HOME": str(cache), "CARGO_TARGET_DIR": str(directory / "cargo-target"),
                           "CARGO_HOME": str(directory / "cargo-home"), "TMPDIR": str(cache)}
            code = 0
            for arguments in commands(request["kind"], working, request["package"], directory):
                entry = {"argv": arguments, "started_at": stamp(), "evidence_kind": "guest-supervision-fixture" if request["kind"] == "supervision-probe" else "real-guest-command"}
                status, out, err, reason = invoke(arguments, working, environment)
                entry.update(upstream_exit=status, finished_at=stamp(), termination_reason=reason)
                report["commands"].append(entry)
                with (directory / "log.txt").open("a", encoding="utf-8") as log:
                    log.write("COMMAND " + json.dumps(arguments) + "\n" + sanitize(out.decode(errors="replace")) + sanitize(err.decode(errors="replace")) + "\n")
                (directory / "log.txt").chmod(0o600)
                atomic(directory / "report.json", report)
                if reason or status:
                    code = 7 if reason == "timeout" else 6
                    break
                if request["kind"] in {"build-system", "build-desktop-test"}:
                    evidence_path = directory / "system-build.json"
                    info = evidence_path.lstat()
                    if not evidence_path.is_file() or evidence_path.is_symlink() or info.st_uid != os.getuid() or info.st_nlink != 1 or info.st_mode & 0o777 != 0o600 or info.st_size > 768 * 1024:
                        raise ValueError("unsafe system build evidence")
                    evidence = source.decode(evidence_path.read_bytes())
                    output = evidence.get("built_output")
                    if evidence.get("state") != "succeeded" or evidence.get("source_digest") != request["snapshot_digest"] or evidence.get("identity") != request["identity"] or evidence.get("activation_performed") is not False or not isinstance(output, str) or not re.fullmatch(r"/nix/store/[a-z0-9]{32}-[A-Za-z0-9._+-]+", output) or source.validate_manifest(evidence["candidate_manifest"]) != evidence["candidate_digest"]:
                        raise ValueError("system build evidence differs from its registered job")
                    report["built_outputs"] = [output]
                    report["system_build"] = evidence
                if arguments[:2] == ["nix", "build"]:
                    outputs = source.decode(out)
                    if not isinstance(outputs, list) or not outputs:
                        raise ValueError("invalid Nix build outputs")
                    paths = []
                    for output in outputs:
                        for path in output["outputs"].values():
                            if not isinstance(path, str) or not re.fullmatch(r"/nix/store/[a-z0-9]{32}-[A-Za-z0-9._+-]+", path):
                                raise ValueError("invalid built store path")
                            paths.append(path)
                    report["built_outputs"] = paths
            for name in ("flake.lock", "Cargo.lock"):
                path = working / name
                if path.exists():
                    data = path.read_bytes()
                    if len(data) > 1024**2:
                        raise ValueError("lock exceeds limit")
                    report["lock_hashes"][name] = hashlib.sha256(data).hexdigest()
                    if request["kind"] == "resolve-lock" and code == 0:
                        artifact = directory / name
                        artifact.write_bytes(data)
                        artifact.chmod(0o600)
    except (ValueError, OSError, KeyError, subprocess.SubprocessError) as error:
        report["failure"] = "JOB_VERIFICATION_FAILED:" + type(error).__name__
        code = 8
    report.update(state="cancelled" if cancelled else "succeeded" if code == 0 else "failed", exit_status=6 if cancelled else code, finished_at=stamp())
    atomic(directory / "report.json", report)


def dispatch(request, identity_reader=source.identity):
    if not isinstance(request, dict):
        raise ValueError("job request must be an object")
    common = {"schema_version", "operation", "identity", "job_id"}
    extra = {"kind", "package", "source_root", "snapshot_digest"} if request.get("operation") == "start" else set()
    if set(request) != common | extra or type(request["schema_version"]) is not int or request["schema_version"] != 1 or request["operation"] not in {"start", "status", "cancel", "pull"}:
        raise ValueError("invalid job request")
    actual = identity_reader()
    if not isinstance(actual, dict) or actual != request["identity"] or actual.get("os_id") != "nixos" or actual.get("guest_role") not in ("development", "acceptance") or actual.get("management_channel") != "ssh-development":
        return 4, {"schema_version": 1, "error": "JOB_TARGET_MISMATCH"}
    directory = job_root() / job_id(request["job_id"])
    if request["operation"] == "start":
        release, manifest = validate_start(request)
        if release is None:
            return 3, {"schema_version": 1, "error": "SOURCE_LOCKS_REQUIRED"}
        try:
            directory.mkdir(mode=0o700)
        except FileExistsError:
            record = read_record(directory)
            if record["request"] != request:
                raise ValueError("job ID already has a different request")
            return 0, record
        record = {"schema_version": 1, "job_id": request["job_id"], "request": request, "state": "queued", "created_at": stamp(),
                  "created_epoch": time.time(), "source_head": manifest["git_head"], "source_dirty": manifest["dirty"], "snapshot_digest": request["snapshot_digest"],
                  "identity": actual, "commands": [], "lock_hashes": {}, "runtime_hash": None, "model_hash": None,
                  "evidence_kind": "real-guest-supervision-fixture" if request["kind"] == "supervision-probe" else "real-guest-development", "limitations": ["This report does not establish final OS, desktop, provider or CPU model acceptance."]}
        atomic(directory / "report.json", record)
        for name, code in (("worker.py", globals().get("_REGISTERED_JOB_SOURCE") or Path(__file__).read_bytes()),
                           ("snapshot.py", getattr(source, "_REGISTERED_SOURCE", None) or Path(source.__file__).read_bytes())):
            (directory / name).write_bytes(code)
            (directory / name).chmod(0o444)
        with (directory / "worker.log").open("xb") as log:
            subprocess.Popen(["/run/current-system/sw/bin/python3", str(directory / "worker.py"), "--worker", str(directory)], stdin=subprocess.DEVNULL, stdout=log, stderr=log, start_new_session=True, close_fds=True)
        (directory / "worker.log").chmod(0o600)
        return 0, record
    record = read_record(directory)
    if any(record["identity"][key] != actual[key] for key in ("dmi_uuid", "installation_uuid", "guest_role", "machine_id", "disk_serial", "management_channel")):
        return 4, {"schema_version": 1, "error": "JOB_TARGET_MISMATCH"}
    if record["state"] not in TERMINAL:
        if record["state"] == "running" and not process_matches(record) or record["state"] == "queued" and time.time() - record["created_epoch"] > 15:
            record.update(state="interrupted", exit_status=6, finished_at=stamp(), failure="WORKER_NOT_LIVE")
            atomic(directory / "report.json", record)
    if request["operation"] == "cancel" and record["state"] not in TERMINAL:
        if not process_matches(record):
            return 3, {"schema_version": 1, "error": "WORKER_NOT_READY"}
        descriptor = os.pidfd_open(record["worker_pid"])
        try:
            if not process_matches(record):
                raise ValueError("worker process changed")
            signal.pidfd_send_signal(descriptor, signal.SIGTERM)
        finally:
            os.close(descriptor)
        return 0, {"schema_version": 1, "job_id": request["job_id"], "state": "cancellation-requested", "identity": actual}
    if request["operation"] == "pull":
        if record["state"] not in TERMINAL:
            return 3, {"schema_version": 1, "error": "JOB_NOT_FINISHED"}
        log = directory / "log.txt"
        data = log.read_bytes() if log.exists() else b""
        if len(data) > 8 * 1024**2:
            raise ValueError("job log exceeds limit")
        result = {"schema_version": 1, "report": record, "sanitized_log": sanitize(data.decode(errors="replace")), "locks": {}}
        if record["request"]["kind"] == "resolve-lock" and record["state"] == "succeeded":
            for name in ("flake.lock", "Cargo.lock"):
                data = (directory / name).read_bytes()
                if len(data) > 1024**2 or hashlib.sha256(data).hexdigest() != record["lock_hashes"][name]:
                    raise ValueError("lock artifact differs")
                result["locks"][name] = base64.b64encode(data).decode()
        return 0, result
    return 0, record


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "--worker":
        worker(Path(sys.argv[2]))
        return 0
    try:
        data = sys.stdin.buffer.read(65537)
        if len(data) > 65536:
            raise ValueError("job request exceeds limit")
        code, result = dispatch(source.decode(data))
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError):
        code, result = 8, {"schema_version": 1, "error": "JOB_REQUEST_VERIFICATION_FAILED"}
    print(json.dumps(result, sort_keys=True))
    return code


if __name__ == "__main__":
    sys.exit(main())
