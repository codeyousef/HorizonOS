#!/usr/bin/env python3
"""Registered unprivileged snapshot receiver. No shell or build execution.

Wire format: eight ASCII decimal length bytes, canonical JSON request, then
exact manifest-ordered file bytes and EOF. All validation precedes publication.
"""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import pwd
import re
import shutil
import stat
import subprocess
import sys
import tempfile

MAX_FILE = 16 * 1024**2
MAX_TOTAL = 64 * 1024**2
MAX_HEADER = 1024**2
MAX_FILES = 10000
MANIFEST = ".aios-source-manifest.json"
PRIVATE_PARTS = {".git", ".local", ".agents", ".aws", ".codex", ".ssh", ".gnupg", ".venv", "__pycache__", "target", "node_modules", "build", "dist", "secrets", "credentials"}
PRIVATE_SUFFIXES = {".key", ".pem", ".p12", ".pfx", ".qcow2", ".iso", ".gguf", ".ggml", ".safetensors", ".onnx", ".pt", ".pth", ".ckpt", ".pyc"}
PRIVATE_NAMES = {"id_rsa", "id_ed25519", "id_ecdsa", "known_hosts", "authorized_keys", ".netrc", ".npmrc", "credentials.json", "token.json", "result"}
PRIVATE_STEMS = {"secret", "secrets", "credential", "credentials", "token", "tokens", "api_key", "api_keys", "api-key", "api-keys"}
KEY_HEADER = re.compile(rb"-----BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY-----")


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()


def pairs(values):
    result = {}
    for key, value in values:
        if key in result:
            raise ValueError("duplicate JSON field")
        result[key] = value
    return result


def decode(data):
    return json.loads(data, object_pairs_hook=pairs, parse_constant=lambda _: (_ for _ in ()).throw(ValueError("non-finite JSON")))


def relative_path(name):
    if not isinstance(name, str) or not name or len(name.encode()) > 4096 or any(ord(c) < 32 or ord(c) == 127 for c in name) or "\\" in name:
        raise ValueError("unsafe source path")
    path = PurePosixPath(name)
    if path.is_absolute() or any(part in (".", "..") for part in path.parts) or str(path) != name:
        raise ValueError("non-normalized source path")
    return path


def excluded(name):
    path = relative_path(name)
    # Public model metadata is source; model weight payloads are excluded.
    model_metadata = len(path.parts) == 2 and path.name in {"manifest.schema.json", "lock.json", "profiles.json"}
    model_template = len(path.parts) > 2 and path.parts[1] == "templates"
    model_payload = path.parts[0] == "models" and not (model_metadata or model_template)
    return model_payload or any(p in PRIVATE_PARTS or p.startswith(".env") or p.startswith("result-") for p in path.parts) or path.name in PRIVATE_NAMES or path.name == MANIFEST or path.stem.lower() in PRIVATE_STEMS or path.suffix.lower() in PRIVATE_SUFFIXES


def validate_manifest(manifest):
    if not isinstance(manifest, dict) or set(manifest) != {"schema_version", "git_head", "dirty", "files"} or type(manifest["schema_version"]) is not int or manifest["schema_version"] != 1:
        raise ValueError("invalid manifest schema")
    if not isinstance(manifest["git_head"], str) or not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", manifest["git_head"]) or type(manifest["dirty"]) is not bool:
        raise ValueError("invalid Git provenance")
    files = manifest["files"]
    if not isinstance(files, list) or not 1 <= len(files) <= MAX_FILES:
        raise ValueError("invalid file count")
    previous, paths, total = "", set(), 0
    for item in files:
        if not isinstance(item, dict) or set(item) != {"path", "mode", "size", "sha256"}:
            raise ValueError("invalid file entry")
        path = relative_path(item["path"])
        if excluded(item["path"]) or item["path"] <= previous or any(str(p) in paths for p in path.parents if str(p) != "."):
            raise ValueError("excluded, duplicate, unsorted or conflicting path")
        if type(item["mode"]) is not int or item["mode"] not in (0o644, 0o755) or type(item["size"]) is not int or not 0 <= item["size"] <= MAX_FILE:
            raise ValueError("unsafe mode or file size")
        if not isinstance(item["sha256"], str) or not re.fullmatch(r"[0-9a-f]{64}", item["sha256"]):
            raise ValueError("invalid content hash")
        previous = item["path"]
        paths.add(previous)
        total += item["size"]
    if total > MAX_TOTAL or len(canonical(manifest)) > MAX_HEADER:
        raise ValueError("snapshot exceeds limit")
    return hashlib.sha256(canonical(manifest)).hexdigest()


def read_regular(root, name, *, published=False):
    """Open every component without following symlinks, including directories."""
    parts = relative_path(name).parts
    descriptor = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        for part in parts[:-1]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = child
        file = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=descriptor)
        with os.fdopen(file, "rb") as handle:
            before = os.fstat(handle.fileno())
            modes = (0o444, 0o555) if published else (0o644, 0o755)
            if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or before.st_uid != os.getuid() or stat.S_IMODE(before.st_mode) not in modes or before.st_size > MAX_FILE:
                raise ValueError("unsafe source file type, mode, ownership or size")
            data = handle.read(MAX_FILE + 1)
            after = os.fstat(handle.fileno())
            if len(data) != before.st_size or (before.st_mtime_ns, before.st_ctime_ns, before.st_size) != (after.st_mtime_ns, after.st_ctime_ns, after.st_size):
                raise ValueError("source changed during collection")
            if KEY_HEADER.search(data):
                raise ValueError("private key material in source")
            mode = 0o755 if before.st_mode & 0o111 else 0o644
            return data, mode
    finally:
        os.close(descriptor)


def verify_tree(root, manifest, *, published=False):
    root_info = root.lstat()
    if not stat.S_ISDIR(root_info.st_mode) or root_info.st_uid != os.getuid() or stat.S_IMODE(root_info.st_mode) != (0o555 if published else 0o700):
        raise ValueError("unsafe snapshot root")
    expected = {item["path"] for item in manifest["files"]} | {MANIFEST}
    expected_dirs = {str(parent) for item in manifest["files"] for parent in PurePosixPath(item["path"]).parents if str(parent) != "."}
    observed = set()
    observed_dirs = set()
    for directory, dirs, files in os.walk(root, followlinks=False):
        for name in dirs:
            observed_dirs.add((Path(directory) / name).relative_to(root).as_posix())
            info = (Path(directory) / name).lstat()
            if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or stat.S_IMODE(info.st_mode) != (0o555 if published else 0o700):
                raise ValueError("unsafe snapshot directory")
        for name in files:
            observed.add((Path(directory) / name).relative_to(root).as_posix())
    if observed != expected or observed_dirs != expected_dirs:
        raise ValueError("unexpected or missing snapshot files")
    metadata, _ = read_regular(root, MANIFEST, published=published)
    if metadata != canonical(manifest):
        raise ValueError("snapshot metadata differs")
    for item in manifest["files"]:
        data, mode = read_regular(root, item["path"], published=published)
        if len(data) != item["size"] or mode != item["mode"] or hashlib.sha256(data).hexdigest() != item["sha256"]:
            raise ValueError("snapshot content differs")


def release_root(name):
    home = Path(pwd.getpwuid(os.getuid()).pw_dir)
    if os.getuid() == 0 or home.parent != Path("/home") or not isinstance(name, str):
        raise ValueError("snapshot receiver requires a normal development user")
    root = Path(name)
    if str(root) != name or not root.is_relative_to(home) or root == home or ".." in root.parts:
        raise ValueError("release root must be under the authenticated user's home")
    for current in [home, *[home / Path(*root.relative_to(home).parts[:i]) for i in range(1, len(root.relative_to(home).parts) + 1)]]:
        if current != home:
            try:
                current.mkdir(mode=0o700)
            except FileExistsError:
                pass
        info = current.lstat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or (current != home and stat.S_IMODE(info.st_mode) != 0o700):
            raise ValueError("release root has unsafe ownership, type or mode")
    return root


def exact(stream, size):
    data = bytearray()
    while len(data) < size:
        chunk = stream.read(min(size - len(data), 65536))
        if not chunk:
            raise ValueError("truncated snapshot")
        data.extend(chunk)
    return bytes(data)


def receive(stream, identity_reader):
    length = exact(stream, 8)
    if not length.isdigit() or not 1 <= int(length) <= MAX_HEADER:
        raise ValueError("invalid request size")
    request = decode(exact(stream, int(length)))
    if not isinstance(request, dict) or set(request) != {"schema_version", "operation", "identity", "source_root", "manifest", "digest"} or type(request["schema_version"]) is not int or request["schema_version"] != 1 or request["operation"] != "publish-source":
        raise ValueError("invalid source request")
    digest = validate_manifest(request["manifest"])
    if request["digest"] != digest:
        raise ValueError("snapshot digest differs")
    identity = identity_reader()
    if not isinstance(identity, dict) or identity != request["identity"] or identity.get("os_id") != "nixos" or identity.get("guest_role") not in ("development", "acceptance") or identity.get("management_channel") != "ssh-development":
        raise ValueError("guest target changed before source mutation")
    root = release_root(request["source_root"])
    stage = Path(tempfile.mkdtemp(prefix=".incoming-", dir=root))
    try:
        for item in request["manifest"]["files"]:
            data = exact(stream, item["size"])
            if hashlib.sha256(data).hexdigest() != item["sha256"] or KEY_HEADER.search(data):
                raise ValueError("transferred content differs or contains private key material")
            target = stage / item["path"]
            directory = stage
            for part in PurePosixPath(item["path"]).parts[:-1]:
                directory = directory / part
                directory.mkdir(exist_ok=True, mode=0o700)
            with target.open("xb") as handle:
                handle.write(data)
                handle.flush()
                os.fsync(handle.fileno())
            target.chmod(item["mode"])
        if stream.read(1):
            raise ValueError("trailing snapshot bytes")
        (stage / MANIFEST).write_bytes(canonical(request["manifest"]))
        (stage / MANIFEST).chmod(0o644)
        verify_tree(stage, request["manifest"])
        for directory, _, files in os.walk(stage, topdown=False):
            for name in files:
                path = Path(directory) / name
                path.chmod(0o555 if path.stat().st_mode & 0o111 else 0o444)
            Path(directory).chmod(0o555)
        destination = root / digest
        if destination.exists() or destination.is_symlink():
            verify_tree(destination, request["manifest"], published=True)
            return {"schema_version": 1, "snapshot_digest": digest, "guest_digest": digest, "guest_source_path": str(destination),
                    "file_count": len(request["manifest"]["files"]), "reused": True, "identity": identity}
        try:
            stage.rename(destination)
            stage = None
            reused = False
        except OSError:
            # Another transfer may have published the same complete digest.
            if not destination.exists() or destination.is_symlink():
                raise
            verify_tree(destination, request["manifest"], published=True)
            reused = True
        verify_tree(destination, request["manifest"], published=True)
        return {"schema_version": 1, "snapshot_digest": digest, "guest_digest": digest, "guest_source_path": str(destination),
                "file_count": len(request["manifest"]["files"]), "reused": reused, "identity": identity}
    finally:
        if stage is not None:
            # Only the unique directory created by this operation is cleaned.
            for directory, _, _ in os.walk(stage):
                Path(directory).chmod(0o700)
            shutil.rmtree(stage)


def identity():
    result = subprocess.run(["/run/current-system/sw/bin/aios-guest-identity"], capture_output=True, timeout=10, check=True)
    if len(result.stdout) > 65536:
        raise ValueError("identity exceeds limit")
    return decode(result.stdout)


def main():
    try:
        result = receive(sys.stdin.buffer, identity)
        print(json.dumps(result, sort_keys=True))
        return 0
    except (ValueError, OSError, subprocess.SubprocessError):
        # Do not leak source, credential content, arbitrary paths or stderr.
        print('{"schema_version":1,"error":"SOURCE_VERIFICATION_FAILED"}')
        return 8


if __name__ == "__main__":
    sys.exit(main())
