"""Explicit console-rooted replacement of a prior installation's SSH trust."""
from dataclasses import replace
import hashlib
import json
import os
import re
import uuid

from . import guest
from .config import VMConfig, invalid, read_json
from .errors import ExitCode
from .provision import digest_file, failure, operation_lock, private_directory, write_json_new, write_new

PENDING = ".local/ssh/re-enrollment-pending.json"
FILES = ("known_hosts", "trust.json", "enrollment.json")
TRUST_FIELDS = {"schema_version", "configuration", "expected", "host_key_fingerprint", "known_hosts_sha256", "source"}


def encoded(value):
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def directory_sync(path):
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def identifier(value):
    try:
        if not isinstance(value, str) or str(uuid.UUID(value)) != value:
            raise ValueError
    except ValueError as error:
        raise invalid("Re-enrollment requires the canonical prior installation UUID") from error
    return value


def destinations(config):
    paths = (config.paths["known_hosts_file"], config.root / ".local/ssh/trust.json", config.root / ".local/enrollment.json")
    if len(set(paths)) != 3 or config.paths["identity_file"] in paths:
        raise invalid("Re-enrollment trust, enrollment and private key paths must be distinct")
    return paths


def prior_enrollment(trust, baseline, previous_installation):
    if (not isinstance(trust, dict) or set(trust) != TRUST_FIELDS
            or type(trust["schema_version"]) is not int or trust["schema_version"] != 1
            or not isinstance(trust["expected"], dict)
            or set(trust["expected"]) != {"guest_uuid", "installation_uuid", "guest_role", "disk_serial", "management_channel"}
            or not isinstance(baseline, dict) or set(baseline) != {"schema_version", "identity", "host_key_fingerprint"}
            or type(baseline["schema_version"]) is not int or baseline["schema_version"] != 1
            or baseline["host_key_fingerprint"] != trust["host_key_fingerprint"]):
        raise invalid("Prior enrollment does not match its pinned trust")
    guest.verify_identity(baseline["identity"], trust["expected"], mutation=True)
    if baseline["identity"]["installation_uuid"] != previous_installation:
        raise failure(ExitCode.TARGET_MISMATCH, "REENROLLMENT_ACKNOWLEDGEMENT_MISMATCH", "Acknowledgement differs from the exact prior enrolled installation")


def replace_file(path, data, operation):
    temporary = path.with_name(path.name + ".re-enrollment-" + operation)
    if temporary.exists() or temporary.is_symlink():
        guest.private_file(temporary)
        if temporary.read_bytes() != data:
            raise invalid("Interrupted re-enrollment replacement changed")
    else:
        write_new(temporary, data)
    os.replace(temporary, path)
    directory_sync(path.parent)


def run(config, previous_installation, trust_file=None):
    previous_installation = identifier(previous_installation)
    with operation_lock(config.root):
        pending = config.root / PENDING
        paths = destinations(config)
        guest.private_file(config.paths["identity_file"])
        if config.values["provider"] == "external":
            if trust_file is None:
                raise failure(ExitCode.UNMET_PREREQUISITE, "CONSOLE_TRUST_REQUIRED", "External re-enrollment requires fresh operator console trust")
            material = guest.external_material(config, trust_file)
        else:
            if trust_file is not None:
                raise invalid("Managed QEMU re-enrollment uses its verified bootstrap console receipt")
            material = guest.console_material(config)
        key, fingerprint, expected, source = material
        if expected["installation_uuid"] == previous_installation:
            raise failure(ExitCode.TARGET_MISMATCH, "REINSTALL_IDENTITY_UNCHANGED", "Re-enrollment requires a distinct new installation UUID")
        request = {"configuration": config.values, "previous_installation_uuid": previous_installation,
                   "console_material_sha256": hashlib.sha256(encoded(material)).hexdigest()}
        if pending.exists() or pending.is_symlink():
            guest.private_file(pending)
            journal = read_json(pending)
            fields = {"schema_version", "operation_id", "request", "previous_trust", "previous_enrollment", "previous_hashes", "staged_hashes"}
            if (not isinstance(journal, dict) or set(journal) != fields or type(journal["schema_version"]) is not int
                    or journal["schema_version"] != 1 or journal["request"] != request
                    or identifier(journal["operation_id"]) != journal["operation_id"]):
                raise failure(ExitCode.TARGET_MISMATCH, "REENROLLMENT_REQUEST_CHANGED", "Resume the same acknowledged console-rooted re-enrollment")
            stage = private_directory(config.root, ".local/ssh/re-enrollments/" + journal["operation_id"])
        else:
            prior_path = config.root / ".local/ssh/trust.json"
            guest.private_file(prior_path)
            prior = read_json(prior_path)
            if not isinstance(prior, dict) or set(prior) != TRUST_FIELDS:
                raise invalid("Invalid prior trust record")
            old_config = VMConfig.from_data(config.root, prior["configuration"])
            prior = guest.load_trust(old_config)
            if old_config.paths["known_hosts_file"] != paths[0]:
                raise invalid("Re-enrollment preserves the configured known_hosts location")
            baseline_path = paths[2]
            guest.private_file(baseline_path)
            baseline = read_json(baseline_path)
            prior_enrollment(prior, baseline, previous_installation)
            operation = str(uuid.uuid4())
            stage = private_directory(config.root, ".local/ssh/re-enrollments/" + operation)
            known = f"[{config.values['ssh_host']}]:{config.values['ssh_port']} {key}\n".encode()
            write_new(stage / FILES[0], known)
            probe = replace(config, paths={**config.paths, "known_hosts_file": stage / FILES[0]})
            value = guest.verify_identity(guest.identity_response(probe), expected, mutation=True)
            trust = {"schema_version": 1, "configuration": config.values, "expected": expected,
                     "host_key_fingerprint": fingerprint, "known_hosts_sha256": hashlib.sha256(known).hexdigest(), "source": source}
            write_json_new(stage / FILES[1], trust)
            write_json_new(stage / FILES[2], {"schema_version": 1, "identity": value, "host_key_fingerprint": fingerprint})
            directory_sync(stage)
            directory_sync(stage.parent)
            journal = {"schema_version": 1, "operation_id": operation, "request": request, "previous_trust": prior,
                       "previous_enrollment": baseline, "previous_hashes": [digest_file(p) for p in paths],
                       "staged_hashes": [digest_file(stage / name) for name in FILES]}
            write_json_new(pending, journal)
            directory_sync(pending.parent)
        if (not isinstance(journal["previous_hashes"], list) or not isinstance(journal["staged_hashes"], list)
                or len(journal["previous_hashes"]) != 3 or len(journal["staged_hashes"]) != 3
                or any(not isinstance(h, str) or not re.fullmatch("[0-9a-f]{64}", h)
                       for h in journal["previous_hashes"] + journal["staged_hashes"])):
            raise invalid("Invalid pending re-enrollment binding")
        prior_enrollment(journal["previous_trust"], journal["previous_enrollment"], previous_installation)
        for name, digest in zip(FILES, journal["staged_hashes"]):
            guest.private_file(stage / name)
            if digest_file(stage / name) != digest:
                raise failure(ExitCode.VERIFICATION_FAILURE, "REENROLLMENT_EVIDENCE_CHANGED", "Staged trust evidence changed")
        staged_trust = read_json(stage / FILES[1])
        staged_enrollment = read_json(stage / FILES[2])
        known = f"[{config.values['ssh_host']}]:{config.values['ssh_port']} {key}\n".encode()
        required_trust = {"schema_version": 1, "configuration": config.values, "expected": expected,
                          "host_key_fingerprint": fingerprint, "known_hosts_sha256": hashlib.sha256(known).hexdigest(), "source": source}
        if staged_trust != required_trust or (stage / FILES[0]).read_bytes() != known:
            raise failure(ExitCode.TARGET_MISMATCH, "REENROLLMENT_EVIDENCE_CHANGED", "Staged trust differs from fresh console material")
        if (set(staged_enrollment) != {"schema_version", "identity", "host_key_fingerprint"}
                or type(staged_enrollment["schema_version"]) is not int or staged_enrollment["schema_version"] != 1
                or staged_enrollment["host_key_fingerprint"] != fingerprint):
            raise invalid("Invalid staged enrollment")
        probe = replace(config, paths={**config.paths, "known_hosts_file": stage / FILES[0]})
        value = guest.verify_identity(guest.identity_response(probe), expected, staged_enrollment["identity"], mutation=True)
        for index, (name, path) in enumerate(zip(FILES, paths)):
            guest.private_file(path)
            if digest_file(path) not in (journal["previous_hashes"][index], journal["staged_hashes"][index]):
                raise failure(ExitCode.TARGET_MISMATCH, "REENROLLMENT_STATE_CHANGED", "Active trust changed outside this re-enrollment")
        for name, path in zip(FILES, paths):
            replace_file(path, (stage / name).read_bytes(), journal["operation_id"])
        receipt = stage / "receipt.json"
        evidence = {**journal, "state": "re-enrolled", "prior_deployment_intents_invalidated": True,
                    "old_authorizations_restored": False, "guest_mutation_performed": False}
        if receipt.exists():
            guest.private_file(receipt)
            if read_json(receipt) != evidence:
                raise invalid("Re-enrollment receipt changed")
        else:
            write_json_new(receipt, evidence)
        directory_sync(stage)
        pending.unlink()
        directory_sync(pending.parent)
        return ExitCode.SUCCESS, {"state": "re-enrolled", "identity_verified": True, "identity": value,
                                  "host_key_fingerprint": fingerprint, "previous_installation_uuid": previous_installation,
                                  "prior_deployment_intents_invalidated": True, "guest_mutation_performed": False,
                                  "artifact_path": str(receipt)}
