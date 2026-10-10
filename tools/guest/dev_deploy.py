#!/usr/bin/env python3
"""Installed VM-only developer authority. Registration never activates code.

Developer Nix deployment is guest-root authority, not a product capability.
Test and commit use an independently retained guard with timeout rollback.
"""
import contextlib
import hashlib
import fcntl
import json
import os
from pathlib import Path
import pwd
import re
import shutil
import sqlite3
import stat
import subprocess
import socket
import sys
import tempfile
import time
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


def verify_target(request, config, actual, expected_system=None):
    validate_identity(actual)
    expected = dict(request["identity"])
    expected["current_system"] = expected_system or expected["current_system"]
    if actual != expected or actual["os_id"] != "nixos" or actual["guest_role"] != "development" or actual["dmi_uuid"] != config["expected_vm_uuid"] or actual["installation_uuid"] != config["expected_installation_uuid"] or actual["disk_serial"] != config["disk_serial"] or actual["management_channel"] != config["management_channel"]:
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

RECEIPT_KEYS = {
    "schema_version", "transaction_id", "state", "snapshot_digest", "identity",
    "developer_uid", "authority", "source_head", "source_dirty", "file_count",
    "activation_performed", "helper_sha256", "limitations", "candidate_closure",
    "candidate_digest", "build_source_digest", "test_guard_id", "commit_guard_id",
    "baseline", "committed_identity",
}


@contextlib.contextmanager
def deployment_lock(state, owner_uid):
    path = state / "deploy.lock"
    if not path.exists():
        descriptor = os.open(path, os.O_RDWR | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    else:
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != owner_uid or info.st_nlink != 1 or stat.S_IMODE(info.st_mode) != 0o600:
            raise ValueError("unsafe deployment lock")
        descriptor = os.open(path, os.O_RDWR | os.O_NOFOLLOW)
    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX)
        yield
    finally:
        os.close(descriptor)


def atomic_file(path, value, mode=0o600):
    data = snapshot.canonical(value)
    temporary = path.with_name("." + path.name + "." + uuid.uuid4().hex)
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
    try:
        with os.fdopen(descriptor, "wb", closefd=False) as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
    finally:
        os.close(descriptor)
    temporary.rename(path)
    fsync_directory(path.parent)


def initial_receipt(request, manifest, developer_uid):
    return {
        "schema_version": 1,
        "transaction_id": request["transaction_id"],
        "state": "REGISTERED",
        "snapshot_digest": request["snapshot_digest"],
        "identity": request["identity"],
        "developer_uid": developer_uid,
        "authority": AUTHORITY,
        "source_head": manifest["git_head"],
        "source_dirty": manifest["dirty"],
        "file_count": len(manifest["files"]),
        "activation_performed": False,
        "helper_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "limitations": ["No activation has occurred; test and commit remain required."],
        "candidate_closure": None,
        "candidate_digest": None,
        "build_source_digest": None,
        "test_guard_id": None,
        "commit_guard_id": None,
        "baseline": None,
        "committed_identity": None,
    }


def validate_receipt(receipt, prior):
    if (not isinstance(receipt, dict) or set(receipt) != RECEIPT_KEYS
            or type(receipt["schema_version"]) is not int or receipt["schema_version"] != 1
            or receipt["transaction_id"] != prior["transaction_id"]
            or receipt["snapshot_digest"] != prior["snapshot_digest"]
            or receipt["identity"] != prior["identity"]
            or receipt["authority"] != AUTHORITY
            or type(receipt["developer_uid"]) is not int or receipt["developer_uid"] <= 0
            or type(receipt["source_dirty"]) is not bool
            or type(receipt["file_count"]) is not int or receipt["file_count"] <= 0
            or type(receipt["activation_performed"]) is not bool
            or not isinstance(receipt["limitations"], list)
            or receipt["state"] not in {"REGISTERED", "TESTING", "TESTED", "COMMITTING", "COMMITTED", "REJECTED", "RECOVERY_REQUIRED"}):
        raise ValueError("invalid registration receipt")
    empty_candidate = receipt["candidate_closure"] is None and receipt["candidate_digest"] is None and receipt["build_source_digest"] is None and receipt["baseline"] is None
    if receipt["state"] == "REGISTERED":
        if not empty_candidate or receipt["test_guard_id"] is not None or receipt["commit_guard_id"] is not None or receipt["activation_performed"] or receipt["committed_identity"] is not None:
            raise ValueError("invalid registered receipt")
    else:
        if (not isinstance(receipt["candidate_closure"], str) or not STORE.fullmatch(receipt["candidate_closure"])
                or not isinstance(receipt["candidate_digest"], str) or not HASH.fullmatch(receipt["candidate_digest"])
                or not isinstance(receipt["build_source_digest"], str) or not HASH.fullmatch(receipt["build_source_digest"])
                or not isinstance(receipt["baseline"], dict)
                or set(receipt["baseline"]) != {"running", "profile", "booted"}
                or not all(isinstance(value, str) and STORE.fullmatch(value) for value in receipt["baseline"].values())):
            raise ValueError("invalid built receipt")
    for field in ("test_guard_id", "commit_guard_id"):
        if receipt[field] is not None:
            canonical_uuid(receipt[field])
    if receipt["state"] in {"TESTING", "TESTED", "COMMITTING", "COMMITTED", "REJECTED", "RECOVERY_REQUIRED"} and receipt["test_guard_id"] is None:
        raise ValueError("missing test guard")
    if receipt["state"] in {"COMMITTING", "COMMITTED"} and receipt["commit_guard_id"] is None:
        raise ValueError("missing commit guard")
    if receipt["state"] in {"REJECTED", "RECOVERY_REQUIRED"} and receipt["activation_performed"] and receipt["commit_guard_id"] is None:
        raise ValueError("qualified test failure requires a commit guard")
    if receipt["state"] in {"TESTED", "COMMITTING", "COMMITTED"} and not receipt["activation_performed"]:
        raise ValueError("completed guard activation evidence missing")
    if receipt["state"] == "COMMITTED":
        validate_identity(receipt["committed_identity"])
        if not receipt["activation_performed"] or receipt["committed_identity"]["current_system"] != receipt["candidate_closure"]:
            raise ValueError("invalid committed receipt")
    elif receipt["committed_identity"] is not None:
        raise ValueError("unexpected committed identity")
    return receipt


def registration(state, owner_uid, transaction_id):
    with ledger(state, owner_uid) as connection:
        row = connection.execute("SELECT request, receipt FROM registrations WHERE id = ?", (transaction_id,)).fetchone()
    if row is None:
        return None
    if len(row[0]) > MAX_REQUEST or len(row[1]) > MAX_REQUEST:
        raise ValueError("registration record exceeds limit")
    prior, receipt = snapshot.decode(row[0]), snapshot.decode(row[1])
    validate_request(prior)
    if prior["operation"] != "register":
        raise ValueError("invalid registration request")
    return prior, validate_receipt(receipt, prior)


def save_receipt(state, owner_uid, prior, receipt):
    validate_receipt(receipt, prior)
    with ledger(state, owner_uid) as connection:
        changed = connection.execute(
            "UPDATE registrations SET receipt = ? WHERE id = ? AND request = ?",
            (snapshot.canonical(receipt), prior["transaction_id"], snapshot.canonical(prior)),
        ).rowcount
        if changed != 1:
            raise ValueError("registration changed")


def safe_remove_tree(path):
    if not path.exists():
        return
    if path.is_symlink():
        raise ValueError("unsafe build directory")
    for directory, directories, _ in os.walk(path):
        Path(directory).chmod(0o700)
        for name in directories:
            child = Path(directory) / name
            if not child.is_symlink():
                child.chmod(0o700)
    shutil.rmtree(path)


def fixed_environment():
    return {
        "HOME": "/root",
        "LANG": "C.UTF-8",
        "NIX_REMOTE": "daemon",
        "NIX_USER_CONF_FILES": "/dev/null",
        "PATH": "/run/current-system/sw/bin:/nix/var/nix/profiles/default/bin",
    }


def build_candidate(state, prior, actual):
    import build_system
    builds = state / "builds"
    builds.mkdir(mode=0o700, exist_ok=True)
    build_root = builds / prior["transaction_id"]
    report_path = build_root / "build.json"
    if report_path.exists():
        info = report_path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_nlink != 1 or stat.S_IMODE(info.st_mode) != 0o600 or info.st_size > MAX_REQUEST:
            raise ValueError("unsafe build receipt")
        report = snapshot.decode(report_path.read_bytes())
        if (not isinstance(report, dict) or set(report) != {"schema_version", "snapshot_digest", "build_source_digest", "candidate_closure", "candidate_digest", "baseline"}
                or report["schema_version"] != 1 or report["snapshot_digest"] != prior["snapshot_digest"]
                or not STORE.fullmatch(report["candidate_closure"]) or not Path(report["candidate_closure"]).is_dir()
                or not HASH.fullmatch(report["candidate_digest"]) or not HASH.fullmatch(report["build_source_digest"])):
            raise ValueError("invalid build receipt")
        return report
    if build_root.exists() or build_root.is_symlink():
        safe_remove_tree(build_root)
    stage = Path(tempfile.mkdtemp(prefix=".build-", dir=builds))
    try:
        release = state / "releases" / prior["snapshot_digest"]
        descriptor = open_directory(release, 0, 0o555)
        try:
            manifest, digest = inspect_source(descriptor, 0)
        finally:
            os.close(descriptor)
        if digest != prior["snapshot_digest"]:
            raise ValueError("registered source digest changed")
        enrolled = build_system.enrollment(actual, build_system.read_enrolled_key())
        _, build_source_digest = build_system.prepare(release, stage / "source", manifest, enrolled)
        space = os.statvfs("/nix/store")
        if space.f_bavail * space.f_frsize < build_system.RESERVE_BYTES:
            raise Denial(3, "DEVELOPMENT_RECOVERY_RESERVE_UNAVAILABLE")
        arguments = build_system.build_arguments(stage / "source", stage / "candidate-root", "aios-dev")
        arguments[0] = "/run/current-system/sw/bin/nix"
        result = subprocess.run(arguments, capture_output=True, timeout=3600, check=False, env=fixed_environment())
        if result.returncode:
            raise Denial(8, "DEVELOPER_BUILD_FAILED")
        outputs = snapshot.decode(result.stdout)
        if len(outputs) != 1 or set(outputs[0].get("outputs", {})) != {"out"}:
            raise ValueError("unexpected developer build output")
        candidate = build_system.store_path(outputs[0]["outputs"]["out"])
        for name, expected in (("installation-uuid", actual["installation_uuid"]), ("expected-dmi-uuid", actual["dmi_uuid"]), ("guest-role", "development")):
            if (Path(candidate) / "etc/aios" / name).read_text().strip() != expected:
                raise ValueError("built candidate enrollment differs")
        report = {
            "schema_version": 1,
            "snapshot_digest": prior["snapshot_digest"],
            "build_source_digest": build_source_digest,
            "candidate_closure": candidate,
            "candidate_digest": hashlib.sha256(candidate.encode()).hexdigest(),
            "baseline": build_system.pointers(),
        }
        atomic_file(stage / "build.json", report)
        stage.rename(build_root)
        stage = None
        fsync_directory(builds)
        return report
    finally:
        if stage is not None:
            safe_remove_tree(stage)


def guard_identity(identity):
    return {
        "installation_uuid": identity["installation_uuid"],
        "dmi_uuid": identity["dmi_uuid"],
        "machine_id": identity["machine_id"],
        "boot_id": identity["boot_id"],
        "role": identity["guest_role"],
        "disk_serial": identity["disk_serial"],
        "management_channel": identity["management_channel"],
    }


def guard_id(registration_id, operation):
    return str(uuid.uuid5(uuid.UUID(registration_id), "aios-dev-guard:" + operation))


def write_handoff(state, prior, receipt, operation, actual):
    identifier = guard_id(prior["transaction_id"], operation)
    handoffs = state / "handoffs"
    handoffs.mkdir(mode=0o700, exist_ok=True)
    descriptor = open_directory(handoffs, 0, 0o700)
    os.close(descriptor)
    path = handoffs / (identifier + ".json")
    value = {
        "schema_version": 1,
        "transaction_id": identifier,
        "registration_id": prior["transaction_id"],
        "developer_uid": receipt["developer_uid"],
        "operation": operation,
        "identity": guard_identity(actual),
        "source_digest": prior["snapshot_digest"],
        "candidate_digest": receipt["candidate_digest"],
        "candidate_closure": receipt["candidate_closure"],
    }
    if path.exists():
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_nlink != 1 or stat.S_IMODE(info.st_mode) != 0o600 or snapshot.decode(path.read_bytes()) != value:
            raise ValueError("developer guard handoff changed")
    else:
        atomic_file(path, value)
    return identifier


def guard_state(identifier, path=Path("/var/lib/aios/transactions/ledger.sqlite"), owner_uid=0):
    if not path.exists():
        return None
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != owner_uid or info.st_nlink != 1 or stat.S_IMODE(info.st_mode) != 0o600:
        raise ValueError("unsafe guard ledger")
    connection = sqlite3.connect("file:" + str(path) + "?mode=ro", uri=True, timeout=0.1)
    try:
        table = connection.execute(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='guard_transactions')"
        ).fetchone()
        if table != (1,):
            return None
        row = connection.execute("SELECT state FROM guard_transactions WHERE id = ?", (identifier,)).fetchone()
        return row[0] if row else None
    finally:
        connection.close()


def receive_exact(connection, length):
    data = bytearray()
    while len(data) < length:
        part = connection.recv(length - len(data))
        if not part:
            raise OSError("guard control closed")
        data.extend(part)
    return bytes(data)


def guard_exchange(identifier, value):
    path = "/run/aios-dev-guard/" + identifier + "/control.sock"
    data = snapshot.canonical(value)
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
        connection.settimeout(2)
        connection.connect(path)
        connection.sendall(len(data).to_bytes(4, "big") + data)
        size = int.from_bytes(receive_exact(connection, 4), "big")
        if not 0 < size <= MAX_REQUEST:
            raise ValueError("invalid guard response length")
        return snapshot.decode(receive_exact(connection, size))


def complete_guard(identifier, operation, receipt):
    terminal = guard_state(identifier)
    expected = "ROLLED_BACK" if operation == "test" else "COMMITTED"
    if terminal == expected:
        return terminal
    if terminal in {"REJECTED", "RECOVERY_REQUIRED"}:
        raise Denial(8, "DEVELOPER_GUARD_FAILED")
    unit = "aios-dev-guard@" + identifier + ".service"
    subprocess.run(
        ["/run/current-system/sw/bin/systemctl", "start", "--no-block", unit],
        check=True, timeout=20, env=fixed_environment(), stdout=subprocess.DEVNULL,
    )
    deadline = time.monotonic() + 300
    while time.monotonic() < deadline:
        state = guard_state(identifier)
        if state == expected:
            return state
        if state in {"REJECTED", "RECOVERY_REQUIRED"}:
            raise Denial(8, "DEVELOPER_GUARD_FAILED")
        try:
            status = guard_exchange(identifier, {"operation": "status", "transaction_id": identifier})
        except (FileNotFoundError, ConnectionRefusedError, socket.timeout, OSError):
            time.sleep(0.1)
            continue
        if (not isinstance(status, dict) or status.get("schema_version") != 1
                or status.get("transaction_id") != identifier
                or status.get("state") != "VERIFYING"
                or status.get("candidate_digest") != receipt["candidate_digest"]
                or status.get("identity") != guard_identity(receipt["identity"])
                or not isinstance(status.get("plan_digest"), str) or not HASH.fullmatch(status["plan_digest"])
                or not isinstance(status.get("nonce"), str) or not re.fullmatch(r"[0-9a-f]{64}", status["nonce"])):
            raise ValueError("developer guard status differs")
        if operation == "test":
            response = guard_exchange(identifier, {"operation": "complete_test", "transaction_id": identifier})
        else:
            response = guard_exchange(identifier, {"operation": "heartbeat", "heartbeat": {
                "schema_version": 1,
                "transaction_id": identifier,
                "identity": status["identity"],
                "plan_digest": status["plan_digest"],
                "candidate_digest": status["candidate_digest"],
                "nonce": status["nonce"],
            }})
        if not isinstance(response, dict) or response.get("schema_version") != 1 or response.get("state") != expected:
            raise ValueError("developer guard terminal response differs")
        return expected
    raise Denial(9, "DEVELOPER_GUARD_TIMEOUT")


def dispatch(request, *, config_reader=read_config, identity_reader=root_identity, caller_reader=caller_uid,
             releases=RELEASES, state=STATE, owner_uid=0):
    validate_request(request)
    developer_uid = caller_reader()
    config = config_reader()
    actual = identity_reader()
    verify_target(request, config, actual)
    state_directory(state, owner_uid)
    with deployment_lock(state, owner_uid):
        found = registration(state, owner_uid, request["transaction_id"])
        if found is None:
            if request["operation"] != "register":
                raise Denial(3, "REGISTRATION_NOT_FOUND")
            verify_target(request, config, actual)
            candidates = state / "releases"
            candidates.mkdir(mode=0o700, exist_ok=True)
            descriptor = open_directory(candidates, owner_uid, 0o700)
            os.close(descriptor)
            manifest = import_release(
                releases,
                candidates / request["snapshot_digest"],
                request["snapshot_digest"],
                developer_uid,
                owner_uid,
            )
            verify_target(request, config_reader(), identity_reader())
            receipt = initial_receipt(request, manifest, developer_uid)
            with ledger(state, owner_uid) as connection:
                connection.execute(
                    "INSERT INTO registrations VALUES (?, ?, ?)",
                    (request["transaction_id"], snapshot.canonical(request), snapshot.canonical(receipt)),
                )
            return receipt

        prior, receipt = found
        if receipt["developer_uid"] != developer_uid:
            raise Denial(4, "REGISTRATION_TARGET_CHANGED")
        if request["operation"] == "status":
            if request["identity"] != prior["identity"]:
                raise Denial(8, "REGISTRATION_REQUEST_CHANGED")
        else:
            expected = dict(prior)
            expected["operation"] = request["operation"]
            if request != expected:
                raise Denial(8, "REGISTRATION_REQUEST_CHANGED")
        durable_guard = None
        if receipt["state"] == "TESTING":
            durable_guard = guard_state(receipt["test_guard_id"])
        elif receipt["state"] == "COMMITTING":
            durable_guard = guard_state(receipt["commit_guard_id"])
        candidate_active = durable_guard in {"VERIFYING", "COMMITTED"}
        expected_system = receipt["candidate_closure"] if receipt["state"] == "COMMITTED" or candidate_active else prior["identity"]["current_system"]
        verify_target(prior, config, actual, expected_system)
        candidate = state / "releases" / prior["snapshot_digest"]
        descriptor = open_directory(candidate, owner_uid, 0o555)
        try:
            manifest, digest = inspect_source(descriptor, owner_uid)
            if (digest != prior["snapshot_digest"]
                    or manifest["git_head"] != receipt["source_head"]
                    or manifest["dirty"] != receipt["source_dirty"]
                    or len(manifest["files"]) != receipt["file_count"]):
                raise ValueError("registered candidate changed")
        finally:
            os.close(descriptor)
        if durable_guard in {"REJECTED", "RECOVERY_REQUIRED"}:
            receipt.update(
                state=durable_guard,
                limitations=[
                    "Independent guard rejected the transaction; no retry or commit is permitted."
                    if durable_guard == "REJECTED" else
                    "Independent guard requires recovery; prior pointers alone do not prove recovery and further activation is blocked."
                ],
            )
            save_receipt(state, owner_uid, prior, receipt)
        elif receipt["state"] == "TESTING" and durable_guard == "ROLLED_BACK":
            receipt.update(
                state="TESTED",
                activation_performed=True,
                limitations=["Candidate test activation passed and rolled back; commit remains required."],
            )
            save_receipt(state, owner_uid, prior, receipt)
        elif receipt["state"] == "COMMITTING" and durable_guard == "COMMITTED":
            import build_system
            pointers = build_system.pointers()
            if (pointers["running"] != receipt["candidate_closure"]
                    or pointers["profile"] != receipt["candidate_closure"]
                    or pointers["booted"] != receipt["baseline"]["booted"]):
                raise ValueError("reconciled committed system pointers differ")
            receipt.update(
                state="COMMITTED",
                activation_performed=True,
                committed_identity=actual,
                limitations=[],
            )
            save_receipt(state, owner_uid, prior, receipt)


        if request["operation"] in {"status", "register"}:
            return receipt
        if request["operation"] == "test":
            if receipt["state"] == "TESTED":
                return receipt
            if receipt["state"] not in {"REGISTERED", "TESTING"}:
                raise Denial(3, "DEPLOYMENT_STATE_INVALID")
            if receipt["state"] == "REGISTERED":
                verify_target(prior, config_reader(), identity_reader())
                built = build_candidate(state, prior, actual)
                verify_target(prior, config_reader(), identity_reader())
                receipt.update(
                    state="TESTING",
                    candidate_closure=built["candidate_closure"],
                    candidate_digest=built["candidate_digest"],
                    build_source_digest=built["build_source_digest"],
                    baseline=built["baseline"],
                    test_guard_id=guard_id(prior["transaction_id"], "test"),
                    limitations=["Guarded test activation is in progress; retry the transaction for durable status."],
                )
                save_receipt(state, owner_uid, prior, receipt)
            actual = identity_reader()
            active = guard_state(receipt["test_guard_id"]) == "VERIFYING"
            verify_target(prior, config_reader(), actual, receipt["candidate_closure"] if active else None)
            identifier = write_handoff(state, prior, receipt, "test", actual)
            if identifier != receipt["test_guard_id"]:
                raise ValueError("test guard identity differs")
            verify_target(prior, config_reader(), identity_reader(), receipt["candidate_closure"] if active else None)
            complete_guard(identifier, "test", receipt)
            verify_target(prior, config_reader(), identity_reader())
            receipt.update(
                state="TESTED",
                activation_performed=True,
                limitations=["Candidate test activation passed and rolled back; commit remains required."],
            )
            save_receipt(state, owner_uid, prior, receipt)
            return receipt

        if receipt["state"] == "COMMITTED":
            return receipt
        if receipt["state"] not in {"TESTED", "COMMITTING"}:
            raise Denial(3, "DEPLOYMENT_STATE_INVALID")
        import build_system
        pointers = build_system.pointers()
        active = receipt["state"] == "COMMITTING" and guard_state(receipt["commit_guard_id"]) == "VERIFYING"
        if not active and pointers != receipt["baseline"]:
            raise Denial(4, "REGISTRATION_TARGET_CHANGED")
        if receipt["state"] == "TESTED":
            receipt.update(
                state="COMMITTING",
                commit_guard_id=guard_id(prior["transaction_id"], "commit"),
                limitations=["Guarded exact-closure commit is in progress; retry the transaction for durable status."],
            )
            save_receipt(state, owner_uid, prior, receipt)
        actual = identity_reader()
        verify_target(prior, config_reader(), actual, receipt["candidate_closure"] if active else None)
        identifier = write_handoff(state, prior, receipt, "commit", actual)
        if identifier != receipt["commit_guard_id"]:
            raise ValueError("commit guard identity differs")
        verify_target(prior, config_reader(), identity_reader(), receipt["candidate_closure"] if active else None)
        complete_guard(identifier, "commit", receipt)
        after = identity_reader()
        verify_target(prior, config_reader(), after, receipt["candidate_closure"])
        pointers = build_system.pointers()
        if (pointers["running"] != receipt["candidate_closure"]
                or pointers["profile"] != receipt["candidate_closure"]
                or pointers["booted"] != receipt["baseline"]["booted"]):
            raise ValueError("committed system pointers differ")
        receipt.update(
            state="COMMITTED",
            activation_performed=True,
            committed_identity=after,
            limitations=[],
        )
        save_receipt(state, owner_uid, prior, receipt)
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
