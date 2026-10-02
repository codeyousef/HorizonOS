#!/usr/bin/env python3
"""Installed VM-only developer authority. Registration never activates code.

Developer Nix deployment is guest-root authority, not a product capability.
The guard adapter is deliberately unavailable until independently qualified.
"""
import contextlib
import hashlib
import json
import os
from pathlib import Path
import pwd
import re
import shutil
import sqlite3
import stat
import subprocess
import sys
import tempfile
import uuid

# Installed code is immutable; Python is invoked with -I by its Nix wrapper.
sys.path.insert(0, str(Path(__file__).resolve().parent))
import snapshot

CONFIG = Path("/etc/aios/development.json")
STATE = Path("/var/lib/aios/development")
RELEASES = Path("/home/dev/aios-releases")
AUTHORITY = "guest-root-code-deployment"
IDENTITY_KEYS = {"schema_version", "os_id", "os_version", "hostname", "dmi_uuid", "installation_uuid", "guest_role", "boot_id", "machine_id", "current_system", "disk_serial", "management_channel"}
MAX_REQUEST = 65536
UUID = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
HASH = re.compile(r"[0-9a-f]{64}")
STORE = re.compile(r"/nix/store/[a-z0-9]{32}-[A-Za-z0-9._+-]+")


class Denial(Exception):
    def __init__(self, code, label):
        self.code, self.label = code, label


def canonical_uuid(value):
    if not isinstance(value, str) or not UUID.fullmatch(value) or str(uuid.UUID(value)) != value:
        raise ValueError("invalid canonical UUID")
    return value


def validate_identity(value):
    if not isinstance(value, dict) or set(value) != IDENTITY_KEYS or type(value["schema_version"]) is not int or value["schema_version"] != 1:
        raise ValueError("invalid target identity")
    for key in IDENTITY_KEYS - {"schema_version"}:
        if not isinstance(value[key], str) or not value[key] or len(value[key]) > 256 or any(ord(c) < 32 or ord(c) == 127 for c in value[key]):
            raise ValueError("invalid target identity field")
    for key in ("dmi_uuid", "installation_uuid", "boot_id"):
        canonical_uuid(value[key])
    if not re.fullmatch(r"[0-9a-f]{32}", value["machine_id"]) or not STORE.fullmatch(value["current_system"]):
        raise ValueError("invalid target machine/closure")
    return value


def validate_request(value):
    common = {"schema_version", "operation", "identity", "transaction_id"}
    mutation = value.get("operation") != "status" if isinstance(value, dict) else False
    fields = common | ({"snapshot_digest", "authority"} if mutation else set())
    if not isinstance(value, dict) or set(value) != fields or type(value["schema_version"]) is not int or value["schema_version"] != 1 or value["operation"] not in {"register", "status", "test", "commit"}:
        raise ValueError("invalid developer request")
    canonical_uuid(value["transaction_id"])
    validate_identity(value["identity"])
    if mutation and (value["authority"] != AUTHORITY or not isinstance(value["snapshot_digest"], str) or not HASH.fullmatch(value["snapshot_digest"])):
        raise ValueError("developer authority acknowledgement/digest required")
    return value


def read_config():
    # NixOS owns this exact /etc link and its immutable store target. No client
    # path, environment override or user-written config is accepted.
    for installed in (Path("/etc"), CONFIG.parent, CONFIG):
        info = installed.lstat()
        if info.st_uid != 0:
            raise ValueError("development configuration link is not root-owned")
        if installed != CONFIG:
            directory = installed.stat()
            if not stat.S_ISDIR(directory.st_mode) or directory.st_uid != 0 or directory.st_mode & 0o022:
                raise ValueError("development configuration parent is writable")
    target = CONFIG.resolve(strict=True)
    if not target.is_relative_to("/nix/store"):
        raise ValueError("development configuration is not immutable")
    # Nix optimisation legitimately hardlinks root-owned readonly store files.
    # Developer source/link checks below remain stricter because dev owns them.
    fd = os.open(target, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as handle:
        info = os.fstat(handle.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o222 or info.st_size > MAX_REQUEST:
            raise ValueError("unsafe development configuration")
        value = snapshot.decode(handle.read(MAX_REQUEST + 1))
    if not isinstance(value, dict) or set(value) != {"schema_version", "enabled", "expected_vm_uuid", "expected_installation_uuid", "disk_serial", "management_channel", "developer_user"} or type(value["schema_version"]) is not int or value["schema_version"] != 1 or value["enabled"] is not True or value["developer_user"] != "dev" or value["disk_serial"] != "AIOS_DEV_ROOT" or value["management_channel"] != "ssh-development":
        raise ValueError("invalid installed development configuration")
    canonical_uuid(value["expected_vm_uuid"])
    canonical_uuid(value["expected_installation_uuid"])
    return value


def caller_uid():
    if os.getuid() != 0 or os.geteuid() != 0 or os.environ.get("SUDO_USER") != "dev":
        raise Denial(5, "DEVELOPER_AUTHORITY_REQUIRED")
    expected = pwd.getpwnam("dev")
    if expected.pw_uid == 0 or expected.pw_dir != "/home/dev" or os.environ.get("SUDO_UID") != str(expected.pw_uid) or os.environ.get("SUDO_GID") != str(expected.pw_gid):
        raise Denial(5, "DEVELOPER_AUTHORITY_REQUIRED")
    # Account enrollment is administrator owned, never supplied in the request.
    return expected.pw_uid


def root_identity():
    virtualization = subprocess.run(["/run/current-system/sw/bin/systemd-detect-virt", "--vm"], capture_output=True, timeout=10, check=False)
    if virtualization.returncode != 0 or virtualization.stdout.strip() not in (b"kvm", b"qemu"):
        raise Denial(4, "DEVELOPMENT_VM_REQUIRED")
    value = validate_identity(snapshot.identity())
    # Root observes real DMI as well as the published unprivileged endpoint.
    actual_dmi = Path("/sys/class/dmi/id/product_uuid").read_text().strip().lower()
    if actual_dmi != value["dmi_uuid"]:
        raise Denial(4, "DEVELOPMENT_TARGET_MISMATCH")
    return value


def verify_target(request, config, actual):
    validate_identity(actual)
    if actual != request["identity"] or actual["os_id"] != "nixos" or actual["guest_role"] != "development" or actual["dmi_uuid"] != config["expected_vm_uuid"] or actual["installation_uuid"] != config["expected_installation_uuid"] or actual["disk_serial"] != config["disk_serial"] or actual["management_channel"] != config["management_channel"]:
        raise Denial(4, "DEVELOPMENT_TARGET_MISMATCH")


def open_directory(path, uid, mode):
    # Pin every ancestor without following symlinks, including a replaced home.
    path = Path(path)
    if not path.is_absolute() or ".." in path.parts:
        raise ValueError("invalid fixed directory")
    fd = os.open("/", os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        for part in path.parts[1:]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd)
            fd = child
        info = os.fstat(fd)
        if info.st_uid != uid or stat.S_IMODE(info.st_mode) != mode:
            raise ValueError("unsafe directory identity/mode")
        return fd
    except BaseException:
        os.close(fd)
        raise


def read_source(root_fd, name, uid):
    parts = snapshot.relative_path(name).parts
    fd = os.dup(root_fd)
    try:
        for part in parts[:-1]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd)
            fd = child
            info = os.fstat(fd)
            if info.st_uid != uid or stat.S_IMODE(info.st_mode) != 0o555:
                raise ValueError("unsafe source directory")
        file = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=fd)
        with os.fdopen(file, "rb") as handle:
            before = os.fstat(handle.fileno())
            if not stat.S_ISREG(before.st_mode) or before.st_uid != uid or before.st_nlink != 1 or stat.S_IMODE(before.st_mode) not in (0o444, 0o555) or before.st_size > snapshot.MAX_FILE:
                raise ValueError("unsafe source file")
            data = handle.read(snapshot.MAX_FILE + 1)
            after = os.fstat(handle.fileno())
            if len(data) != before.st_size or (before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (after.st_size, after.st_mtime_ns, after.st_ctime_ns) or snapshot.KEY_HEADER.search(data):
                raise ValueError("source changed or contains key material")
            return data, 0o755 if before.st_mode & 0o111 else 0o644
    finally:
        os.close(fd)


def inspect_source(root_fd, uid):
    metadata, _ = read_source(root_fd, snapshot.MANIFEST, uid)
    if len(metadata) > snapshot.MAX_HEADER:
        raise ValueError("source metadata exceeds limit")
    manifest = snapshot.decode(metadata)
    digest = snapshot.validate_manifest(manifest)
    if metadata != snapshot.canonical(manifest):
        raise ValueError("source metadata is not canonical")
    expected_files = {item["path"] for item in manifest["files"]} | {snapshot.MANIFEST}
    expected_dirs = {str(parent) for item in manifest["files"] for parent in Path(item["path"]).parents if str(parent) != "."}
    observed_files, observed_dirs = set(), set()
    pending = [(os.dup(root_fd), "")]
    try:
        while pending:
            directory_fd, prefix = pending.pop()
            try:
                # Iterate rather than collecting an unbounded caller-owned tree.
                # Unknown entries stop traversal before they can grow a worklist.
                with os.scandir(directory_fd) as entries:
                    for entry in entries:
                        name = prefix + entry.name
                        info = entry.stat(follow_symlinks=False)
                        if stat.S_ISDIR(info.st_mode):
                            if name not in expected_dirs or info.st_uid != uid or stat.S_IMODE(info.st_mode) != 0o555:
                                raise ValueError("unsafe or unlisted source directory")
                            child = os.open(entry.name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=directory_fd)
                            child_info = os.fstat(child)
                            if (child_info.st_dev, child_info.st_ino) != (info.st_dev, info.st_ino):
                                os.close(child)
                                raise ValueError("source directory replaced")
                            pending.append((child, name + "/"))
                            observed_dirs.add(name)
                        else:
                            if name not in expected_files or not stat.S_ISREG(info.st_mode):
                                raise ValueError("unsafe or unlisted source file")
                            observed_files.add(name)
            finally:
                os.close(directory_fd)
    finally:
        for descriptor, _ in pending:
            os.close(descriptor)
    if expected_files != observed_files or expected_dirs != observed_dirs:
        raise ValueError("source tree has extra or missing entries")
    for item in manifest["files"]:
        data, mode = read_source(root_fd, item["path"], uid)
        if mode != item["mode"] or len(data) != item["size"] or hashlib.sha256(data).hexdigest() != item["sha256"]:
            raise ValueError("source content differs")
    return manifest, digest


def fsync_directory(path):
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def seal_stage(stage):
    for directory, _, files in os.walk(stage, topdown=False):
        for name in files:
            path = Path(directory) / name
            path.chmod(0o555 if path.stat().st_mode & 0o111 else 0o444)
        Path(directory).chmod(0o555)
        fsync_directory(directory)


def remove_stage(stage):
    for directory, _, _ in os.walk(stage):
        Path(directory).chmod(0o700)
    shutil.rmtree(stage)


def import_release(releases, destination, digest, developer_uid, owner_uid):
    # Source is user-owned even when readonly. Hash while copying into a fresh
    # root-owned tree; never build/activate directly from that user's directory.
    root_fd = open_directory(releases, developer_uid, 0o700)
    try:
        source_fd = os.open(digest, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=root_fd)
    finally:
        os.close(root_fd)
    stage = None
    try:
        info = os.fstat(source_fd)
        if info.st_uid != developer_uid or stat.S_IMODE(info.st_mode) != 0o555:
            raise ValueError("unsafe published release")
        manifest, observed = inspect_source(source_fd, developer_uid)
        if observed != digest:
            raise ValueError("registered source digest differs")
        # A root-owned state directory makes destination races unavailable to
        # the caller. Existing content is independently reverified before reuse.
        if destination.exists() or destination.is_symlink():
            existing_fd = open_directory(destination, owner_uid, 0o555)
            try:
                existing, actual = inspect_source(existing_fd, owner_uid)
                if existing != manifest or actual != digest:
                    raise ValueError("root candidate content differs")
            finally:
                os.close(existing_fd)
            return manifest
        stage = Path(tempfile.mkdtemp(prefix=".incoming-", dir=destination.parent))
        for item in manifest["files"]:
            data, mode = read_source(source_fd, item["path"], developer_uid)
            if len(data) != item["size"] or mode != item["mode"] or hashlib.sha256(data).hexdigest() != item["sha256"]:
                raise ValueError("source changed while copying")
            target = stage / item["path"]
            target.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            with target.open("xb") as handle:
                handle.write(data)
                handle.flush()
                os.fsync(handle.fileno())
            target.chmod(mode)
        # The copied metadata is the original manifest, never recomputed to bless
        # caller modifications. Verify again before atomic publication.
        target = stage / snapshot.MANIFEST
        with target.open("xb") as handle:
            handle.write(snapshot.canonical(manifest))
            handle.flush()
            os.fsync(handle.fileno())
        seal_stage(stage)
        staged_fd = open_directory(stage, owner_uid, 0o555)
        try:
            copied, actual = inspect_source(staged_fd, owner_uid)
            if copied != manifest or actual != digest:
                raise ValueError("candidate verification differs")
        finally:
            os.close(staged_fd)
        stage.rename(destination)
        stage = None
        fsync_directory(destination.parent)
        return manifest
    finally:
        os.close(source_fd)
        if stage is not None:
            remove_stage(stage)


def state_directory(path, owner_uid):
    # /var/lib/aios is root-owned; parent substitution is denied before writes.
    fd = open_directory(path.parent, owner_uid, 0o755)
    try:
        try:
            os.mkdir(path.name, 0o700, dir_fd=fd)
            os.fsync(fd)
        except FileExistsError:
            pass
        info = os.stat(path.name, dir_fd=fd, follow_symlinks=False)
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != owner_uid or stat.S_IMODE(info.st_mode) != 0o700:
            raise ValueError("unsafe development state")
    finally:
        os.close(fd)
    return path


@contextlib.contextmanager
def ledger(state, owner_uid):
    path = state / "registrations.sqlite"
    if path.exists() or path.is_symlink():
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != owner_uid or info.st_nlink != 1 or stat.S_IMODE(info.st_mode) != 0o600:
            raise ValueError("unsafe registration ledger")
    else:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        os.close(fd)
        fsync_directory(state)
    connection = sqlite3.connect(path, timeout=0.1)
    try:
        connection.execute("PRAGMA synchronous=FULL")
        connection.execute("PRAGMA journal_mode=DELETE")
        connection.execute("CREATE TABLE IF NOT EXISTS registrations (id TEXT PRIMARY KEY, request BLOB NOT NULL, receipt BLOB NOT NULL)")
        connection.execute("BEGIN IMMEDIATE")
        yield connection
        connection.commit()
        fsync_directory(state)
    except BaseException:
        connection.rollback()
        raise
    finally:
        connection.close()


def dispatch(request, *, config_reader=read_config, identity_reader=root_identity, caller_reader=caller_uid,
             releases=RELEASES, state=STATE, owner_uid=0):
    validate_request(request)
    developer_uid = caller_reader()
    config = config_reader()
    actual = identity_reader()
    verify_target(request, config, actual)
    if request["operation"] in {"test", "commit"}:
        # No unguarded deployment shortcut. The qualified independent activation
        # adapter must replace this denial before the public modes can succeed.
        raise Denial(9, "GUARDED_ACTIVATION_UNAVAILABLE")
    state_directory(state, owner_uid)
    with ledger(state, owner_uid) as connection:
        row = connection.execute("SELECT request, receipt FROM registrations WHERE id = ?", (request["transaction_id"],)).fetchone()
        if row:
            if len(row[0]) > MAX_REQUEST or len(row[1]) > MAX_REQUEST:
                raise ValueError("registration record exceeds limit")
            prior, receipt = snapshot.decode(row[0]), snapshot.decode(row[1])
            validate_request(prior)
            if not isinstance(receipt, dict) or prior["operation"] != "register" or receipt.get("state") != "REGISTERED" or receipt.get("transaction_id") != request["transaction_id"] or receipt.get("snapshot_digest") != prior["snapshot_digest"] or receipt.get("activation_performed") is not False or receipt.get("identity") != prior["identity"]:
                raise ValueError("invalid registration receipt")
            if prior["identity"] != actual or receipt["developer_uid"] != developer_uid:
                raise Denial(4, "REGISTRATION_TARGET_CHANGED")
            if request["operation"] != "status" and prior != request:
                raise Denial(8, "REGISTRATION_REQUEST_CHANGED")
            candidate = state / "releases" / prior["snapshot_digest"]
            candidate_fd = open_directory(candidate, owner_uid, 0o555)
            try:
                manifest, digest = inspect_source(candidate_fd, owner_uid)
                if digest != prior["snapshot_digest"] or manifest["git_head"] != receipt.get("source_head") or manifest["dirty"] != receipt.get("source_dirty") or len(manifest["files"]) != receipt.get("file_count"):
                    raise ValueError("registered candidate changed")
            finally:
                os.close(candidate_fd)
            return receipt
        if request["operation"] == "status":
            raise Denial(3, "REGISTRATION_NOT_FOUND")
        candidates = state / "releases"
        candidates.mkdir(mode=0o700, exist_ok=True)
        fd = open_directory(candidates, owner_uid, 0o700)
        os.close(fd)
        manifest = import_release(releases, candidates / request["snapshot_digest"], request["snapshot_digest"], developer_uid, owner_uid)
        # Re-observe immediately before the durable registration mutation too.
        verify_target(request, config_reader(), identity_reader())
        receipt = {"schema_version": 1, "transaction_id": request["transaction_id"], "state": "REGISTERED",
                   "snapshot_digest": request["snapshot_digest"], "identity": actual, "developer_uid": developer_uid,
                   "authority": AUTHORITY, "source_head": manifest["git_head"], "source_dirty": manifest["dirty"],
                   "file_count": len(manifest["files"]), "activation_performed": False,
                   "helper_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                   "limitations": ["Guarded test activation and exact-closure commit are unavailable.", "Registration is not build, activation, production isolation or final OS acceptance."]}
        connection.execute("INSERT INTO registrations VALUES (?, ?, ?)", (request["transaction_id"], snapshot.canonical(request), snapshot.canonical(receipt)))
        return receipt


def main():
    os.umask(0o077)
    try:
        if sys.argv[1:] != ["--request-stdin"]:
            raise Denial(2, "INVALID_DEVELOPER_REQUEST")
        # Authentication before input prevents an unprivileged caller from
        # using the parser/filesystem as a privileged read or mutation route.
        caller_uid()
        data = sys.stdin.buffer.read(MAX_REQUEST + 1)
        if len(data) > MAX_REQUEST:
            raise Denial(2, "INVALID_DEVELOPER_REQUEST")
        try:
            request = validate_request(snapshot.decode(data))
        except (ValueError, TypeError, KeyError, UnicodeError):
            raise Denial(2, "INVALID_DEVELOPER_REQUEST")
        result = dispatch(request)
        print(snapshot.canonical(result).decode())
        return 0
    except Denial as error:
        print(snapshot.canonical({"schema_version": 1, "error": error.label}).decode())
        return error.code
    except (ValueError, TypeError, KeyError, UnicodeError):
        print('{"schema_version":1,"error":"DEVELOPER_VERIFICATION_FAILED"}')
        return 8
    except (OSError, sqlite3.Error, subprocess.SubprocessError):
        print('{"schema_version":1,"error":"DEVELOPER_REGISTRATION_FAILED"}')
        return 8


if __name__ == "__main__":
    sys.exit(main())
