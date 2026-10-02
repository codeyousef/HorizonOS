"""Console-rooted SSH trust and bounded, read-only guest identity verification."""
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import stat
import subprocess
import time
import uuid

from .config import invalid, object_without_duplicates, project_path, read_json
from .errors import DevctlError, ExitCode
from .provision import DISK_SERIAL, digest_file, failure, private_directory, write_json_new, write_new

IDENTITY_COMMAND = "/run/current-system/sw/bin/aios-guest-identity"
IDENTITY_FIELDS = {"schema_version", "os_id", "os_version", "hostname", "dmi_uuid", "installation_uuid", "guest_role", "boot_id", "machine_id", "current_system", "disk_serial", "management_channel"}


def private_file(path):
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_nlink != 1 or stat.S_IMODE(info.st_mode) != 0o600:
        raise invalid("SSH trust and key files must be owned regular files with mode 0600")


def public_key(value):
    if not isinstance(value, str) or not value or len(value) > 1024:
        raise invalid("Invalid console host public key")
    words = value.strip().split()
    if len(words) < 2 or words[0] != "ssh-ed25519":
        raise invalid("A console-verified ed25519 public host key is required")
    try:
        raw = base64.b64decode(words[1], validate=True)
    except ValueError as error:
        raise invalid("Malformed SSH host public key") from error
    if len(raw) != 51 or raw[:19] != b"\x00\x00\x00\x0bssh-ed25519\x00\x00\x00\x20":
        raise invalid("Malformed ed25519 public key encoding")
    fingerprint = "SHA256:" + base64.b64encode(hashlib.sha256(raw).digest()).decode().rstrip("=")
    return " ".join(words[:2]), fingerprint


def pin_console(config):
    from .vm import load_record
    record = load_record(config)
    receipt_path = project_path(config.root, ".local/vm/bootstrap-result.json", ".local/vm")
    if not receipt_path.exists():
        raise failure(ExitCode.UNMET_PREREQUISITE, "BOOTSTRAP_INCOMPLETE", "A successful verified console bootstrap is required before pinning SSH trust")
    private_file(receipt_path)
    receipt = read_json(receipt_path)
    plan = record["plan"]
    for field, expected in (("schema_version", 1), ("upstream_exit", 0), ("guest_uuid", plan["guest_uuid"]),
                            ("installation_uuid", plan["installation_uuid"]), ("seed_sha256", record["seed_sha256"]), ("media_sha256", record["media"]["sha256"])):
        if type(receipt.get(field)) is not type(expected) or receipt.get(field) != expected:
            raise failure(ExitCode.TARGET_MISMATCH, "CONSOLE_TRUST_MISMATCH", "Bootstrap receipt does not match this installation/media")
    relative = Path(receipt["serial_path"]).relative_to(config.root)
    path = project_path(config.root, str(relative), ".local/vm/console")
    private_file(path)
    if path.stat().st_size > 64 * 1024 * 1024 or digest_file(path) != receipt["serial_sha256"]:
        raise failure(ExitCode.VERIFICATION_FAILURE, "CONSOLE_EVIDENCE_MISMATCH", "Console evidence was changed")
    text = path.read_text().replace("\r", "")
    expected = f"Verified NixOS installer, virt=kvm, DMI={plan['guest_uuid']}, role=development, disk=/dev/vda, serial={DISK_SERIAL}"
    if expected not in text or f"installation UUID={plan['installation_uuid']}" not in text or not text.rstrip().endswith("AIOS_BOOTSTRAP_EXIT=0"):
        raise failure(ExitCode.VERIFICATION_FAILURE, "CONSOLE_EVIDENCE_MISMATCH", "Missing successful target-verified bootstrap evidence")
    keys = re.findall(r"^ssh-ed25519 [A-Za-z0-9+/=]+(?: [^\n]*)?$", text, re.MULTILINE)
    if len(keys) != 1:
        raise failure(ExitCode.VERIFICATION_FAILURE, "CONSOLE_KEY_AMBIGUOUS", "Expected exactly one console host public key")
    key, fingerprint = public_key(keys[0])
    if fingerprint not in text:
        raise failure(ExitCode.VERIFICATION_FAILURE, "CONSOLE_FINGERPRINT_MISMATCH", "Console key/fingerprint disagree")
    return install_trust(config, key, fingerprint, {
        "guest_uuid": plan["guest_uuid"], "installation_uuid": plan["installation_uuid"], "guest_role": "development",
        "disk_serial": DISK_SERIAL, "management_channel": "ssh-development",
    }, {"kind": "verified-qemu-console", "serial_sha256": receipt["serial_sha256"]})


def install_trust(config, key, fingerprint, expected, source):
    private_directory(config.root, ".local/ssh")
    private_file(config.paths["identity_file"])
    known = config.paths["known_hosts_file"]
    contents = f"[{config.values['ssh_host']}]:{config.values['ssh_port']} {key}\n".encode()
    if known.exists():
        private_file(known)
        if known.read_bytes() != contents:
            raise failure(ExitCode.TARGET_MISMATCH, "SSH_HOST_KEY_MISMATCH", "Existing SSH trust differs; explicit re-enrollment is required")
    else:
        write_new(known, contents)
    trust = {"schema_version": 1, "configuration": config.values, "expected": expected,
             "host_key_fingerprint": fingerprint, "known_hosts_sha256": digest_file(known), "source": source}
    path = config.root / ".local/ssh/trust.json"
    if path.exists():
        private_file(path)
        if read_json(path) != trust:
            raise failure(ExitCode.TARGET_MISMATCH, "SSH_TRUST_MISMATCH", "Existing enrollment trust differs")
    else:
        write_json_new(path, trust)
    return trust


def load_trust(config):
    path = project_path(config.root, ".local/ssh/trust.json", ".local/ssh")
    if not path.exists():
        raise failure(ExitCode.UNMET_PREREQUISITE, "GUEST_NOT_ENROLLED", "Guest has no console-rooted SSH trust")
    private_file(path)
    trust = read_json(path)
    private_file(config.paths["identity_file"])
    private_file(config.paths["known_hosts_file"])
    fields = {"schema_version", "configuration", "expected", "host_key_fingerprint", "known_hosts_sha256", "source"}
    if not isinstance(trust, dict) or set(trust) != fields or type(trust.get("schema_version")) is not int or trust.get("schema_version") != 1 or trust.get("configuration") != config.values or trust.get("known_hosts_sha256") != digest_file(config.paths["known_hosts_file"]):
        raise failure(ExitCode.TARGET_MISMATCH, "SSH_TRUST_MISMATCH", "SSH target configuration or pinned host key changed")
    return trust


def pin_external(config, relative):
    if config.values["provider"] != "external":
        raise invalid("Operator console trust files are for external-provider adoption")
    path = project_path(config.root, relative, ".local/ssh")
    private_file(path)
    value = read_json(path)
    fields = {"schema_version", "host_public_key", "fingerprint", "guest_uuid", "installation_uuid", "guest_role", "disk_serial", "management_channel"}
    if not isinstance(value, dict) or set(value) != fields or type(value["schema_version"]) is not int or value["schema_version"] != 1:
        raise invalid("Invalid operator console trust schema")
    key, fingerprint = public_key(value["host_public_key"])
    if fingerprint != value["fingerprint"]:
        raise failure(ExitCode.TARGET_MISMATCH, "CONSOLE_FINGERPRINT_MISMATCH", "Operator console key/fingerprint disagree")
    for field in ("guest_uuid", "installation_uuid"):
        try:
            if not isinstance(value[field], str) or str(uuid.UUID(value[field])) != value[field]:
                raise ValueError
        except ValueError as error:
            raise invalid("Invalid operator console UUID") from error
    if value["guest_role"] not in ("development", "acceptance", "recovery") or value["management_channel"] != "ssh-development" or not isinstance(value["disk_serial"], str) or not re.fullmatch(r"[A-Za-z0-9_.-]{1,128}", value["disk_serial"]):
        raise invalid("Invalid operator console role/disk/management identity")
    for field in ("guest_uuid", "installation_uuid", "guest_role"):
        if field in config.values and config.values[field] != value[field]:
            raise failure(ExitCode.TARGET_MISMATCH, "EXTERNAL_TARGET_MISMATCH", "Configured identity differs from operator console trust")
    expected = {field: value[field] for field in ("guest_uuid", "installation_uuid", "guest_role", "disk_serial", "management_channel")}
    return install_trust(config, key, fingerprint, expected, {"kind": "operator-console", "trust_material_sha256": digest_file(path)})


def ssh_arguments(config):
    options = {"BatchMode": "yes", "IdentitiesOnly": "yes", "IdentityAgent": "none", "StrictHostKeyChecking": "yes",
               "UserKnownHostsFile": str(config.paths["known_hosts_file"]), "GlobalKnownHostsFile": "/dev/null", "UpdateHostKeys": "no",
               "ForwardAgent": "no", "ClearAllForwardings": "yes", "ControlMaster": "no", "ControlPath": "none", "RequestTTY": "no",
               "PasswordAuthentication": "no", "KbdInteractiveAuthentication": "no", "ConnectTimeout": "5"}
    result = ["ssh", "-F", "/dev/null"]
    for name, value in options.items():
        result.extend(["-o", f"{name}={value}"])
    return [*result, "-p", str(config.values["ssh_port"]), "-i", str(config.paths["identity_file"]),
            f"{config.values['ssh_user']}@{config.values['ssh_host']}", IDENTITY_COMMAND]


def enrolled_identity(config):
    """Verify the enrolled target immediately before a registered operation."""
    trust = load_trust(config)
    path = project_path(config.root, ".local/enrollment.json", ".local")
    if not path.exists():
        raise failure(ExitCode.UNMET_PREREQUISITE, "GUEST_NOT_ENROLLED", "Run enroll to verify the pinned guest identity")
    private_file(path)
    baseline = read_json(path)
    value = verify_identity(identity_response(config), trust["expected"], baseline["identity"], mutation=True)
    return trust, value


def identity_response(config):
    # Bounded streams and deadline. Never echo arbitrary SSH errors or guest data
    # in an exception, and never fall back to password/keyscan/disabled checking.
    process = subprocess.Popen(ssh_arguments(config), stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    output, errors = bytearray(), bytearray()
    try:
        with selectors.DefaultSelector() as selector:
            selector.register(process.stdout, selectors.EVENT_READ, output)
            selector.register(process.stderr, selectors.EVENT_READ, errors)
            deadline = time.monotonic() + 15
            while selector.get_map():
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise failure(ExitCode.TIMEOUT, "SSH_IDENTITY_TIMEOUT", "Guest identity endpoint timed out")
                for event, _ in selector.select(min(remaining, 1)):
                    chunk = os.read(event.fd, 4096)
                    if not chunk:
                        selector.unregister(event.fileobj)
                        continue
                    event.data.extend(chunk)
                    if len(output) + len(errors) > 65536:
                        raise failure(ExitCode.VERIFICATION_FAILURE, "IDENTITY_RESPONSE_LIMIT", "Guest identity response exceeds 64 KiB")
        status = process.wait(timeout=1)
        if status:
            reason = "endpoint_failed"
            for message, classification in ((b"Connection refused", "connection_refused"), (b"Permission denied", "authentication_denied"),
                                             (b"Host key verification failed", "host_key_rejected"), (b"HOST IDENTIFICATION HAS CHANGED", "host_key_changed"),
                                             (b"No such file", "endpoint_missing")):
                if message in errors:
                    reason = classification
                    break
            raise DevctlError(ExitCode.TARGET_MISMATCH if reason in ("host_key_changed", "host_key_rejected") else ExitCode.UNMET_PREREQUISITE,
                             "SSH_IDENTITY_FAILED", "Pinned guest SSH identity request failed", details={"upstream_exit": status, "reason": reason})
        try:
            return json.loads(output.decode("utf-8"), object_pairs_hook=object_without_duplicates,
                              parse_constant=lambda _: (_ for _ in ()).throw(ValueError("Non-finite JSON")))
        except (ValueError, UnicodeError, DevctlError) as error:
            raise failure(ExitCode.VERIFICATION_FAILURE, "INVALID_GUEST_IDENTITY", "Guest identity is not valid JSON") from error
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        process.stdout.close()
        process.stderr.close()


def verify_identity(value, expected, baseline=None, *, mutation=False):
    if not isinstance(value, dict) or set(value) != IDENTITY_FIELDS or type(value.get("schema_version")) is not int or value.get("schema_version") != 1:
        raise failure(ExitCode.TARGET_MISMATCH, "GUEST_IDENTITY_MISMATCH", "Guest identity schema mismatch")
    for field in IDENTITY_FIELDS - {"schema_version"}:
        if not isinstance(value[field], str) or not value[field] or len(value[field]) > 512:
            raise failure(ExitCode.TARGET_MISMATCH, "GUEST_IDENTITY_MISMATCH", "Guest identity has invalid fields")
    for field in ("dmi_uuid", "installation_uuid", "boot_id"):
        try:
            if str(uuid.UUID(value[field])) != value[field]:
                raise ValueError
        except ValueError as error:
            raise failure(ExitCode.TARGET_MISMATCH, "GUEST_IDENTITY_MISMATCH", "Guest identity UUID is invalid") from error
    if value["os_id"] != "nixos" or not re.fullmatch(r"[0-9a-f]{32}", value["machine_id"]) or not re.fullmatch(r"/nix/store/[a-z0-9]{32}-nixos-system-[A-Za-z0-9._+-]+", value["current_system"]):
        raise failure(ExitCode.TARGET_MISMATCH, "GUEST_IDENTITY_MISMATCH", "Guest OS, machine ID or running closure mismatch")
    for field, wanted in (("dmi_uuid", expected["guest_uuid"]), ("installation_uuid", expected["installation_uuid"]), ("guest_role", expected["guest_role"])):
        if value[field] != wanted:
            raise failure(ExitCode.TARGET_MISMATCH, "GUEST_IDENTITY_MISMATCH", "Guest installation/DMI/role mismatch")
    if baseline and any(value[field] != baseline[field] for field in ("machine_id", "hostname")):
        raise failure(ExitCode.TARGET_MISMATCH, "GUEST_IDENTITY_MISMATCH", "Enrolled guest machine/hostname changed")
    if mutation and any(value[field] != expected[field] for field in ("disk_serial", "management_channel")):
        raise failure(ExitCode.TARGET_MISMATCH, "GUEST_MUTATION_IDENTITY_MISMATCH", "Guest disk/management identity mismatch")
    return value


def doctor(config):
    trust, value = enrolled_identity(config)
    return ExitCode.SUCCESS, {"identity_verified": True, "host_key_fingerprint": trust["host_key_fingerprint"], "identity": value, "read_only": True}


def enroll(config, trust_file=None):
    if trust_file is not None:
        pin_external(config, trust_file)
    trust_path = config.root / ".local/ssh/trust.json"
    if not trust_path.exists():
        if config.values["provider"] != "qemu":
            raise failure(ExitCode.UNMET_PREREQUISITE, "CONSOLE_TRUST_REQUIRED", "External adoption requires explicit console-rooted trust material")
        pin_console(config)
    trust = load_trust(config)
    value = verify_identity(identity_response(config), trust["expected"], mutation=True)
    path = config.root / ".local/enrollment.json"
    if path.exists():
        private_file(path)
        verify_identity(value, trust["expected"], read_json(path)["identity"], mutation=True)
    else:
        write_json_new(path, {"schema_version": 1, "identity": value, "host_key_fingerprint": trust["host_key_fingerprint"]})
    return ExitCode.SUCCESS, {"identity_verified": True, "host_key_fingerprint": trust["host_key_fingerprint"], "identity": value}
