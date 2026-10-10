"""Pinned transport for the installed VM-only developer registration helper."""
import hashlib
import os
import re
import uuid

from . import guest, sync, resources
from .config import invalid
from .errors import DevctlError, ExitCode
from .provision import failure, operation_lock, private_directory, write_new

AUTHORITY = "guest-root-code-deployment"
COMMAND = "/run/wrappers/bin/sudo -n /run/current-system/sw/bin/aios-dev-deploy --request-stdin"
FIELDS = {"schema_version", "transaction_id", "state", "snapshot_digest", "identity", "developer_uid",
          "authority", "source_head", "source_dirty", "file_count", "activation_performed",
          "helper_sha256", "limitations", "candidate_closure", "candidate_digest",
          "build_source_digest", "test_guard_id", "commit_guard_id", "baseline", "committed_identity"}
DENIALS = {"INVALID_DEVELOPER_REQUEST", "DEVELOPER_AUTHORITY_REQUIRED", "DEVELOPMENT_VM_REQUIRED",
           "DEVELOPMENT_TARGET_MISMATCH", "REGISTRATION_TARGET_CHANGED", "REGISTRATION_REQUEST_CHANGED",
           "REGISTRATION_NOT_FOUND", "DEVELOPER_VERIFICATION_FAILED", "DEVELOPER_REGISTRATION_FAILED",
           "DEVELOPMENT_RECOVERY_RESERVE_UNAVAILABLE", "DEVELOPER_BUILD_FAILED",
           "DEPLOYMENT_STATE_INVALID", "DEVELOPER_GUARD_FAILED", "DEVELOPER_GUARD_TIMEOUT"}


def identifier(value):
    try:
        if not isinstance(value, str) or str(uuid.UUID(value)) != value:
            raise ValueError
    except ValueError as error:
        raise invalid("Deployment transaction must be a canonical UUID") from error
    return value


def read_record(path):
    guest.private_file(path)
    if path.stat().st_size > 65536:
        raise invalid("Deployment record exceeds limit")
    try:
        return sync.contract.decode(path.read_bytes())
    except (ValueError, UnicodeError) as error:
        raise failure(ExitCode.VERIFICATION_FAILURE, "INVALID_DEPLOYMENT_RECORD", "Invalid deployment record") from error


def remember(path, value):
    data = sync.contract.canonical(value)
    if path.exists():
        if read_record(path) != value:
            raise failure(ExitCode.VERIFICATION_FAILURE, "DEPLOYMENT_RECORD_CHANGED", "Durable deployment evidence changed")
    else:
        write_new(path, data)
        # The transaction intent must survive a transport disconnect/crash.
        fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)


def validate_receipt(value, intent):
    request = intent["request"]
    if (not isinstance(value, dict) or set(value) != FIELDS or type(value["schema_version"]) is not int
            or value["schema_version"] != 1
            or value["state"] not in {"REGISTERED", "TESTING", "TESTED", "COMMITTING", "COMMITTED"}
            or any(value[key] != request[key] for key in ("transaction_id", "snapshot_digest", "identity", "authority"))
            or value["source_head"] != intent["source_head"] or type(value["source_dirty"]) is not bool
            or value["source_dirty"] != intent["source_dirty"] or type(value["file_count"]) is not int
            or value["file_count"] != intent["file_count"] or type(value["developer_uid"]) is not int
            or value["developer_uid"] <= 0 or type(value["activation_performed"]) is not bool
            or not isinstance(value["helper_sha256"], str) or not re.fullmatch(r"[0-9a-f]{64}", value["helper_sha256"]) or not isinstance(value["limitations"], list)):
        raise failure(ExitCode.VERIFICATION_FAILURE, "DEPLOYMENT_RECEIPT_MISMATCH", "Installed helper receipt differs from the frozen transaction")
    built = value["state"] != "REGISTERED"
    if built:
        if (not isinstance(value["candidate_closure"], str) or not re.fullmatch(r"/nix/store/[a-z0-9]{32}-[A-Za-z0-9._+-]+", value["candidate_closure"])
                or not isinstance(value["candidate_digest"], str) or not re.fullmatch(r"[0-9a-f]{64}", value["candidate_digest"])
                or not isinstance(value["build_source_digest"], str) or not re.fullmatch(r"[0-9a-f]{64}", value["build_source_digest"])
                or not isinstance(value["baseline"], dict) or set(value["baseline"]) != {"running", "profile", "booted"}
                or not all(isinstance(path, str) and re.fullmatch(r"/nix/store/[a-z0-9]{32}-[A-Za-z0-9._+-]+", path) for path in value["baseline"].values())):
            raise failure(ExitCode.VERIFICATION_FAILURE, "DEPLOYMENT_RECEIPT_MISMATCH", "Built deployment evidence is invalid")
    elif any(value[field] is not None for field in ("candidate_closure", "candidate_digest", "build_source_digest", "test_guard_id", "commit_guard_id", "baseline", "committed_identity")) or value["activation_performed"]:
        raise failure(ExitCode.VERIFICATION_FAILURE, "DEPLOYMENT_RECEIPT_MISMATCH", "Registration unexpectedly reports activation")
    for field in ("test_guard_id", "commit_guard_id"):
        if value[field] is not None:
            identifier(value[field])
    if value["state"] in {"TESTING", "TESTED", "COMMITTING", "COMMITTED"} and value["test_guard_id"] is None:
        raise failure(ExitCode.VERIFICATION_FAILURE, "DEPLOYMENT_RECEIPT_MISMATCH", "Test guard evidence is missing")
    if value["state"] in {"COMMITTING", "COMMITTED"} and value["commit_guard_id"] is None:
        raise failure(ExitCode.VERIFICATION_FAILURE, "DEPLOYMENT_RECEIPT_MISMATCH", "Commit guard evidence is missing")
    if value["state"] in {"TESTED", "COMMITTING", "COMMITTED"} and value["activation_performed"] is not True:
        raise failure(ExitCode.VERIFICATION_FAILURE, "DEPLOYMENT_RECEIPT_MISMATCH", "Completed guard activation evidence is missing")
    if value["state"] == "COMMITTED":
        if (not isinstance(value["committed_identity"], dict)
                or not same_target(value["committed_identity"], value["identity"])
                or value["committed_identity"].get("current_system") != value["candidate_closure"]
                or value["activation_performed"] is not True):
            raise failure(ExitCode.VERIFICATION_FAILURE, "DEPLOYMENT_RECEIPT_MISMATCH", "Committed identity differs from the exact candidate")
    elif value["committed_identity"] is not None:
        raise failure(ExitCode.VERIFICATION_FAILURE, "DEPLOYMENT_RECEIPT_MISMATCH", "Unexpected committed identity")
    return value

def same_target(left, right):
    if not isinstance(left, dict) or not isinstance(right, dict) or set(left) != set(right):
        return False
    return all(left[key] == right[key] for key in left if key != "current_system")



def run(config, mode, transaction=None, acknowledge=False):
    if mode not in {"register", "status", "test", "commit"}:
        raise invalid("Invalid deployment mode")
    if mode != "status" and acknowledge is not True:
        raise failure(ExitCode.AUTHORIZATION_NEEDED, "DEVELOPER_AUTHORITY_ACKNOWLEDGEMENT_REQUIRED",
                      "Developer Nix code deployment is guest-root authority; use --acknowledge-guest-root")
    if transaction is None and mode != "register":
        raise invalid("This deployment mode requires --transaction")
    transaction = identifier(transaction) if transaction is not None else str(uuid.uuid4())
    if mode != "status":
        resources.require_build_headroom(config)
    with operation_lock(config.root):
        trust, identity = guest.enrolled_identity(config)
        if (config.values["ssh_user"] != "dev" or config.values["guest_source_root"] != "/home/dev/aios-releases"
                or identity["guest_role"] != "development" or identity["disk_serial"] != "AIOS_DEV_ROOT"
                or identity["management_channel"] != "ssh-development"):
            raise failure(ExitCode.TARGET_MISMATCH, "DEVELOPMENT_TARGET_MISMATCH", "Deployment requires the enrolled development VM and developer account")
        directory = private_directory(config.root, ".local/deployments/" + transaction)
        path = directory / "intent.json"
        if path.exists():
            intent = read_record(path)
        else:
            if mode != "register":
                raise failure(ExitCode.UNMET_PREREQUISITE, "DEPLOYMENT_NOT_REGISTERED", "No host transaction intent exists")
            _, source = sync.synchronize(config)
            _, current = guest.enrolled_identity(config)
            if current != identity or source["identity"] != identity:
                raise failure(ExitCode.TARGET_MISMATCH, "DEPLOYMENT_TARGET_CHANGED", "Development target changed during publication")
            record = config.root / ".local/snapshots" / (source["snapshot_digest"] + ".json")
            guest.private_file(record)
            if record.stat().st_size > sync.contract.MAX_HEADER + 4096:
                raise invalid("Source provenance exceeds limit")
            manifest = sync.contract.decode(record.read_bytes())["manifest"]
            if sync.contract.validate_manifest(manifest) != source["snapshot_digest"]:
                raise failure(ExitCode.VERIFICATION_FAILURE, "DEPLOYMENT_SOURCE_MISMATCH", "Published source provenance changed")
            helper = [item for item in manifest["files"] if item["path"] == "tools/guest/dev_deploy.py"]
            if len(helper) != 1 or helper[0]["sha256"] != hashlib.sha256((config.root / "tools/guest/dev_deploy.py").read_bytes()).hexdigest():
                raise failure(ExitCode.VERIFICATION_FAILURE, "DEPLOYMENT_HELPER_SOURCE_MISMATCH", "Published helper source changed")
            intent = {"schema_version": 1, "configuration": config.values, "host_key_fingerprint": trust["host_key_fingerprint"],
                      "request": {"schema_version": 1, "operation": "register", "transaction_id": transaction,
                                  "identity": identity, "snapshot_digest": source["snapshot_digest"], "authority": AUTHORITY},
                      "source_head": manifest["git_head"], "source_dirty": manifest["dirty"],
                      "file_count": len(manifest["files"]), "helper_sha256": helper[0]["sha256"]}
            remember(path, intent)
        if (not isinstance(intent, dict) or set(intent) != {"schema_version", "configuration", "host_key_fingerprint", "request", "source_head", "source_dirty", "file_count", "helper_sha256"}
                or type(intent["schema_version"]) is not int or intent["schema_version"] != 1
                or intent["configuration"] != config.values or intent["host_key_fingerprint"] != trust["host_key_fingerprint"]
                or not isinstance(intent["request"], dict) or set(intent["request"]) != {"schema_version", "operation", "transaction_id", "identity", "snapshot_digest", "authority"}
                or intent["request"]["schema_version"] != 1 or type(intent["request"]["schema_version"]) is not int
                or intent["request"]["operation"] != "register" or intent["request"]["authority"] != AUTHORITY
                or intent["request"]["transaction_id"] != transaction or not same_target(intent["request"]["identity"], identity)
                or not isinstance(intent["request"]["snapshot_digest"], str) or not re.fullmatch(r"[0-9a-f]{64}", intent["request"]["snapshot_digest"])):
            raise failure(ExitCode.TARGET_MISMATCH, "DEPLOYMENT_TARGET_CHANGED", "Frozen deployment intent no longer matches this target/boot/configuration")
        request = {**intent["request"], "operation": mode}
        if mode == "status":
            request = {key: request[key] for key in ("schema_version", "operation", "identity", "transaction_id")}
        evidence = {"transaction_id": transaction, "intent_path": str(path)}
        try:
            timeout = 3700 if mode == "test" else 600 if mode == "commit" else 120
            status, output, errors = sync.exchange([*guest.ssh_arguments(config)[:-1], COMMAND],
                                                   [sync.contract.canonical(request)], response_limit=65536, timeout=timeout)
            _, after = guest.enrolled_identity(config)
        except DevctlError as error:
            error.details.update(evidence)
            raise
        except TimeoutError as error:
            raise DevctlError(ExitCode.TIMEOUT, "DEVELOPER_TRANSPORT_TIMEOUT", "Developer operation timed out; inspect its durable transaction", details=evidence) from error
        try:
            response = sync.contract.decode(output)
        except (ValueError, UnicodeError):
            response = None
        if status:
            label = response.get("error") if isinstance(response, dict) and set(response) == {"schema_version", "error"} and type(response["schema_version"]) is int and response["schema_version"] == 1 else None
            label = label if label in DENIALS else "DEVELOPER_HELPER_FAILED"
            code = ExitCode(status) if status in range(2, 10) else ExitCode.OPERATION_FAILURE
            if b"Host key verification failed" in errors or b"HOST IDENTIFICATION HAS CHANGED" in errors:
                code = ExitCode.TARGET_MISMATCH
            raise DevctlError(code, label, "Installed developer helper refused the operation",
                              details={"upstream_exit": status, **evidence})
        receipt = validate_receipt(response, intent)
        expected_after = receipt["committed_identity"] if receipt["state"] == "COMMITTED" else receipt["identity"]
        if after != expected_after:
            raise failure(ExitCode.TARGET_MISMATCH, "DEPLOYMENT_TARGET_CHANGED", "Development target changed during helper operation")
        receipts = private_directory(config.root, ".local/deployments/" + transaction + "/receipts")
        artifact = receipts / (mode + "-" + receipt["state"].lower() + ".json")
        remember(artifact, receipt)
        return ExitCode.SUCCESS, {**receipt, "identity_verified": True, "host_key_fingerprint": trust["host_key_fingerprint"],
                                  "release_digest": receipt["snapshot_digest"], "artifact_path": str(artifact)}
