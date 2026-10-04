"""Host control of registered durable jobs in the enrolled NixOS guest."""
import base64
import hashlib
from pathlib import Path
import time
import uuid

from . import guest, sync
from .config import invalid
from .errors import DevctlError, ExitCode
from .provision import failure, private_directory, write_json_new, write_new

SCRIPT = Path(__file__).resolve().parents[1] / "guest/jobs.py"
PACKAGES = ("aios-consent-ui", "aios-core", "aios-model", "aios-desktop", "aios-cli", "aios-dev-tools")
TERMINAL = {"succeeded", "failed", "cancelled", "interrupted"}


def arguments(config):
    # Executed code is the registered public controller, not request content.
    snapshot = base64.b64encode(sync.RECEIVER.read_bytes()).decode()
    jobs = base64.b64encode(SCRIPT.read_bytes()).decode()
    program = ("import base64,types,sys; m=types.ModuleType('snapshot'); m._REGISTERED_SOURCE=base64.b64decode('" + snapshot + "'); "
               "exec(compile(m._REGISTERED_SOURCE,'<aios-snapshot-contract>','exec'),m.__dict__); sys.modules['snapshot']=m; "
               "_REGISTERED_JOB_SOURCE=base64.b64decode('" + jobs + "'); exec(compile(_REGISTERED_JOB_SOURCE,'<aios-job-controller>','exec'))")
    encoded = base64.b64encode(program.encode()).decode()
    command = '/run/current-system/sw/bin/python3 -c "import base64;exec(base64.b64decode(\'' + encoded + '\'))"'
    return [*guest.ssh_arguments(config)[:-1], command]


def identifier(value):
    try:
        if not isinstance(value, str) or str(uuid.UUID(value)) != value:
            raise ValueError
    except ValueError as error:
        raise invalid("Job ID must be a canonical UUID") from error
    return value


def request(config, operation, job, **extra):
    trust, identity = guest.enrolled_identity(config)
    value = {"schema_version": 1, "operation": operation, "identity": identity, "job_id": identifier(job), **extra}
    payload = sync.contract.canonical(value)
    if len(payload) > 65536:
        raise invalid("Job request exceeds limit")
    status, out, err = sync.exchange(arguments(config), [payload], response_limit=16 * 1024**2, timeout=30)
    if status:
        code = ExitCode.TARGET_MISMATCH if b"Host key verification failed" in err or b"HOST IDENTIFICATION HAS CHANGED" in err else ExitCode(status) if status in range(2, 10) else ExitCode.OPERATION_FAILURE
        try:
            response = sync.contract.decode(out)
            label = response.get("error", "GUEST_JOB_FAILED")
        except (ValueError, UnicodeError, AttributeError):
            label = "GUEST_JOB_FAILED"
        if label not in {"SOURCE_LOCKS_REQUIRED", "WORKER_NOT_READY", "JOB_NOT_FINISHED", "JOB_TARGET_MISMATCH", "JOB_REQUEST_VERIFICATION_FAILED"}:
            label = "GUEST_JOB_FAILED"
        raise DevctlError(code, label, "Registered guest job request failed", details={"upstream_exit": status, "job_id": job})
    try:
        response = sync.contract.decode(out)
    except (ValueError, UnicodeError) as error:
        raise failure(ExitCode.VERIFICATION_FAILURE, "INVALID_JOB_RESPONSE", "Invalid guest job response") from error
    if not isinstance(response, dict) or type(response.get("schema_version")) is not int or response.get("schema_version") != 1:
        raise failure(ExitCode.VERIFICATION_FAILURE, "INVALID_JOB_RESPONSE", "Invalid guest job response schema")
    report = response.get("report") if operation == "pull" else response
    if not isinstance(report, dict) or report.get("job_id") != job or report.get("state") not in TERMINAL | {"queued", "running", "cancellation-requested"}:
        raise failure(ExitCode.VERIFICATION_FAILURE, "JOB_RESPONSE_MISMATCH", "Job response does not match the selected job")
    guest.verify_identity(report.get("identity"), trust["expected"], identity, mutation=True)
    if report["state"] != "cancellation-requested":
        pointer = config.root / ".local/jobs" / (job + ".json")
        guest.private_file(pointer)
        if pointer.stat().st_size > 65536:
            raise invalid("Local job pointer exceeds limit")
        expected = sync.contract.decode(pointer.read_bytes())
        original = report.get("request")
        if not isinstance(original, dict) or original.get("job_id") != job or report.get("snapshot_digest") != expected.get("snapshot_digest") or any(original.get(key) != expected.get(key) for key in ("kind", "snapshot_digest", "package")) or original.get("source_root") != config.values["guest_source_root"] or original.get("identity") != report["identity"]:
            raise failure(ExitCode.VERIFICATION_FAILURE, "JOB_PROVENANCE_MISMATCH", "Guest job provenance differs from the host's registered job")
        source_record = config.root / ".local/snapshots" / (expected["snapshot_digest"] + ".json")
        guest.private_file(source_record)
        if source_record.stat().st_size > sync.contract.MAX_HEADER + 4096:
            raise invalid("Local source provenance exceeds limit")
        manifest = sync.contract.decode(source_record.read_bytes())["manifest"]
        if sync.contract.validate_manifest(manifest) != expected["snapshot_digest"] or report.get("source_head") != manifest["git_head"] or type(report.get("source_dirty")) is not bool or report["source_dirty"] != manifest["dirty"]:
            raise failure(ExitCode.VERIFICATION_FAILURE, "JOB_SOURCE_MISMATCH", "Job source metadata differs from the registered manifest")
        if report["state"] in TERMINAL and (type(report.get("exit_status")) is not int or report["exit_status"] not in range(10)):
            raise failure(ExitCode.VERIFICATION_FAILURE, "JOB_STATUS_MISMATCH", "Invalid terminal job exit status")
    return response


def remember(config, job, kind, digest, package):
    directory = private_directory(config.root, ".local/jobs")
    write_json_new(directory / (job + ".json"), {"schema_version": 1, "job_id": job, "configuration": config.values,
                                              "kind": kind, "snapshot_digest": digest, "package": package})


def resolve_job(config, job=None):
    directory = private_directory(config.root, ".local/jobs")
    if job:
        path = directory / (identifier(job) + ".json")
    else:
        paths = sorted(directory.glob("*.json"), key=lambda path: path.stat().st_mtime_ns)
        if not paths:
            raise failure(ExitCode.UNMET_PREREQUISITE, "NO_GUEST_JOB", "No registered guest job exists for this workspace")
        path = paths[-1]
    if not path.exists():
        raise failure(ExitCode.UNMET_PREREQUISITE, "JOB_NOT_REGISTERED", "The requested job is not registered in this workspace")
    guest.private_file(path)
    if path.stat().st_size > 65536:
        raise invalid("Local job pointer exceeds limit")
    record = sync.contract.decode(path.read_bytes())
    if record.get("configuration") != config.values or record.get("job_id") != path.stem:
        raise failure(ExitCode.TARGET_MISMATCH, "JOB_CONFIGURATION_MISMATCH", "Job pointer belongs to another target configuration")
    return identifier(record["job_id"])


def pull(config, job=None):
    job = resolve_job(config, job)
    response = request(config, "pull", job)
    if set(response) != {"schema_version", "report", "sanitized_log", "locks"} or not isinstance(response["sanitized_log"], str) or not isinstance(response["locks"], dict):
        raise failure(ExitCode.VERIFICATION_FAILURE, "INVALID_ARTIFACT_RESPONSE", "Invalid guest artifact response")
    report = response["report"]
    log = response["sanitized_log"].encode()
    if len(log) > 8 * 1024**2:
        raise invalid("Sanitized job log exceeds limit")
    # Fixed filenames, exclusive writes and fresh run directory only.
    directory = private_directory(config.root, ".local/reports/" + job + "-" + str(uuid.uuid4()))
    write_json_new(directory / "report.json", report)
    write_new(directory / "log.txt", log)
    if set(response["locks"]) - {"flake.lock", "Cargo.lock"}:
        raise failure(ExitCode.VERIFICATION_FAILURE, "INVALID_LOCK_ARTIFACT", "Unexpected lock artifact")
    hashes = {}
    for name, encoded in response["locks"].items():
        try:
            data = base64.b64decode(encoded, validate=True)
        except (TypeError, ValueError) as error:
            raise invalid("Invalid lock artifact encoding") from error
        digest = hashlib.sha256(data).hexdigest()
        if len(data) > 1024**2 or digest != report["lock_hashes"].get(name):
            raise failure(ExitCode.VERIFICATION_FAILURE, "LOCK_ARTIFACT_MISMATCH", "Lock artifact does not match the guest report")
        write_new(directory / name, data)
        hashes[name] = digest
    summary = (f"Job: {job}\nState: {report['state']}\nExit: {report.get('exit_status')}\n"
               f"Source: {report.get('snapshot_digest')}\nEvidence: {report.get('evidence_kind')}\n"
               "No final OS, desktop, provider or model acceptance is established by this development job.\n")
    write_new(directory / "summary.txt", summary.encode())
    return ExitCode.SUCCESS, {"job_id": job, "state": report["state"], "job_exit_status": report.get("exit_status"), "release_digest": report.get("snapshot_digest"),
                              "artifact_path": str(directory / "report.json"), "artifact_directory": str(directory),
                              "lock_hashes": report["lock_hashes"], "lock_artifact_hashes": hashes, "identity_verified": True}


def start(config, kind, *, package=None, detach=False, wait_seconds=120):
    if kind not in {"resolve-lock", "build-packages", "build-system", "build-desktop-test", "test-unit", "system-info-smoke", "service-inspection-smoke", "public-session-smoke", "installed-session-inference-smoke", "consent-ui-smoke", "accessibility-smoke", "ui-provider-smoke", "model-compatibility-smoke", "upstream-compatibility-smoke", "host-boundary-smoke", "protocol-conformance-smoke", "model-profile-low-smoke", "model-profile-high-smoke", "model-inference-smoke", "model-service-smoke", "session-inference-smoke", "development-boundary-smoke", "guard-state-smoke", "managed-state-smoke", "broker-preparation-smoke", "installed-runtime-smoke", "installed-policy-smoke", "installed-development-smoke", "installed-guard-smoke", "installed-model-smoke", "installed-model-idle-smoke", "installed-model-lifecycle-smoke", "supervision-probe"} or package is not None and (kind != "build-packages" or package not in PACKAGES):
        raise invalid("Unregistered build/test job")
    _, snapshot = sync.synchronize(config)
    provenance = sync.contract.decode(Path(snapshot["artifact_path"]).read_bytes())
    paths = {item["path"] for item in provenance["manifest"]["files"]}
    if kind != "resolve-lock" and not {"flake.lock", "Cargo.lock"}.issubset(paths):
        raise failure(ExitCode.UNMET_PREREQUISITE, "SOURCE_LOCKS_REQUIRED", "Generate and verify Nix/Cargo locks before submitting a guest build or test")
    job = str(uuid.uuid4())
    # Remember the ID before SSH delivery so a lost reply remains recoverable.
    remember(config, job, kind, snapshot["snapshot_digest"], package)
    response = request(config, "start", job, kind=kind, package=package,
                       source_root=config.values["guest_source_root"], snapshot_digest=snapshot["snapshot_digest"])
    if detach:
        return ExitCode.SUCCESS, {"job_id": job, "state": response["state"], "release_digest": snapshot["snapshot_digest"], "identity_verified": True,
                                  "message": "Job is tracked; use jobs status or jobs cancel with this ID."}
    deadline = time.monotonic() + wait_seconds
    while response["state"] not in TERMINAL:
        if time.monotonic() >= deadline:
            raise DevctlError(ExitCode.TIMEOUT, "JOB_WAIT_TIMEOUT", "Guest job continues with a durable ID; inspect its status", details={"job_id": job})
        time.sleep(1)
        response = request(config, "status", job)
    _, result = pull(config, job)
    status = response.get("exit_status", 6)
    return ExitCode(status), {**result, "upstream_exits": [entry["upstream_exit"] for entry in response["commands"]]}


def control(config, operation, job):
    job = resolve_job(config, job)
    response = request(config, operation, job)
    return ExitCode.SUCCESS, {"job_id": job, "state": response["state"], "job_exit_status": response.get("exit_status"),
                              "release_digest": response.get("snapshot_digest"), "identity_verified": True, "report": response}
