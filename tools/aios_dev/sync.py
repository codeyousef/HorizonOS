"""Content-addressed public source snapshots over the pinned guest transport."""
import base64
import hashlib
import importlib.util
import os
from pathlib import Path
import selectors
import subprocess
import time

from . import guest
from .config import invalid
from .errors import DevctlError, ExitCode
from .provision import failure, private_directory, write_new

# One registered receiver supplied by the host tooling, never a caller command.
RECEIVER = Path(__file__).resolve().parents[1] / "guest/snapshot.py"
spec = importlib.util.spec_from_file_location("aios_snapshot_contract", RECEIVER)
contract = importlib.util.module_from_spec(spec)
spec.loader.exec_module(contract)


def git(root, *arguments):
    result = subprocess.run(["git", "-C", str(root), *arguments], capture_output=True, timeout=10, check=False)
    if result.returncode or len(result.stdout) > 4 * 1024**2:
        raise invalid("Cannot collect bounded Git source metadata")
    return result.stdout


def collect(root):
    head = git(root, "rev-parse", "HEAD").decode().strip()
    status = git(root, "-c", "core.filemode=true", "status", "--porcelain=v1", "-z", "--untracked-files=all")
    names = set(git(root, "ls-files", "-z", "--cached", "--others", "--exclude-standard").decode().split("\0")) - {""}
    deleted = set(git(root, "ls-files", "-z", "--deleted").decode().split("\0")) - {""}
    files, contents, total = [], [], 0
    try:
        for name in sorted(names):
            if contract.excluded(name) or name in deleted:
                continue
            data, mode = contract.read_regular(root, name)
            total += len(data)
            if total > contract.MAX_TOTAL or len(files) >= contract.MAX_FILES:
                raise ValueError("source exceeds snapshot limits")
            files.append({"path": name, "mode": mode, "size": len(data), "sha256": hashlib.sha256(data).hexdigest()})
            contents.append(data)
        manifest = {"schema_version": 1, "git_head": head, "dirty": bool(status), "files": files}
        digest = contract.validate_manifest(manifest)
        # Catch file edits even when Git's dirty status remains unchanged.
        for item, data in zip(files, contents):
            current, mode = contract.read_regular(root, item["path"])
            if current != data or mode != item["mode"]:
                raise ValueError("source changed during snapshot")
        if git(root, "rev-parse", "HEAD").decode().strip() != head or git(root, "-c", "core.filemode=true", "status", "--porcelain=v1", "-z", "--untracked-files=all") != status:
            raise ValueError("Git provenance changed during snapshot")
    except (ValueError, OSError) as error:
        raise invalid("Source snapshot contains an unsafe path, mode, credential or concurrent change") from error
    return manifest, digest, contents


def receiver_arguments(config):
    source = RECEIVER.read_bytes()
    # Encoding contains only public registered code, no keys or caller input.
    encoded = base64.b64encode(source).decode()
    command = '/run/current-system/sw/bin/python3 -c "import base64;exec(compile(base64.b64decode(\'' + encoded + '\'),\'<aios-source-receiver>\',\'exec\'))"'
    return [*guest.ssh_arguments(config)[:-1], command]


def exchange(arguments, payloads, *, response_limit=65536, timeout=120):
    """Bounded transport shared only by registered host operations."""
    payloads = iter(payloads)
    pending = memoryview(next(payloads))
    process = subprocess.Popen(arguments, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    output, errors = bytearray(), bytearray()
    try:
        with selectors.DefaultSelector() as selector:
            for pipe in (process.stdin, process.stdout, process.stderr):
                os.set_blocking(pipe.fileno(), False)
            selector.register(process.stdin, selectors.EVENT_WRITE, "input")
            selector.register(process.stdout, selectors.EVENT_READ, output)
            selector.register(process.stderr, selectors.EVENT_READ, errors)
            deadline = time.monotonic() + timeout
            while selector.get_map():
                if time.monotonic() >= deadline:
                    raise failure(ExitCode.TIMEOUT, "SOURCE_TRANSFER_TIMEOUT", "Guest source transfer timed out")
                for event, _ in selector.select(1):
                    if event.data == "input":
                        if pending:
                            try:
                                count = os.write(event.fd, pending[:65536])
                                pending = pending[count:]
                            except BrokenPipeError:
                                selector.unregister(event.fileobj)
                                process.stdin.close()
                                continue
                        if not pending:
                            try:
                                pending = memoryview(next(payloads))
                            except StopIteration:
                                selector.unregister(event.fileobj)
                                process.stdin.close()
                    else:
                        chunk = os.read(event.fd, 4096)
                        if chunk:
                            event.data.extend(chunk)
                            if len(output) + len(errors) > response_limit:
                                raise failure(ExitCode.VERIFICATION_FAILURE, "SOURCE_RESPONSE_LIMIT", "Source receiver exceeded its response limit")
                        else:
                            selector.unregister(event.fileobj)
        status = process.wait(timeout=1)
        return status, bytes(output), bytes(errors)
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        for pipe in (process.stdin, process.stdout, process.stderr):
            pipe.close()


def transfer(config, request, contents):
    header = contract.canonical(request)
    if len(header) > contract.MAX_HEADER:
        raise invalid("Snapshot request exceeds header limit")
    status, output, errors = exchange(receiver_arguments(config), [f"{len(header):08d}".encode(), header, *contents])
    if status:
        code = ExitCode.TARGET_MISMATCH if b"Host key verification failed" in errors or b"HOST IDENTIFICATION HAS CHANGED" in errors else ExitCode.VERIFICATION_FAILURE
        details = {"upstream_exit": status}
        try:
            refusal = contract.decode(output)
            allowed = {
                "SOURCE_STORAGE_EXHAUSTED", "SOURCE_IO_FAILED", "SOURCE_VERIFICATION_FAILED",
                "SOURCE_UNSAFE_STAGE", "SOURCE_TREE_ENTRIES_MISMATCH",
                "SOURCE_PUBLISHED_MANIFEST_MISMATCH", "SOURCE_STAGED_MANIFEST_MISMATCH", "SOURCE_CONTENT_MISMATCH",
                "SOURCE_UNSAFE_RELEASE_ROOT", "SOURCE_TRUNCATED", "SOURCE_TARGET_CHANGED",
                "SOURCE_TRAILING_DATA", "SOURCE_IDENTITY_FAILED",
            }
            fields = set(refusal) if isinstance(refusal, dict) else set()
            if (fields == {"schema_version", "error"} and refusal["schema_version"] == 1 and refusal["error"] in allowed):
                details["receiver_error"] = refusal["error"]
            elif (fields == {"schema_version", "error", "expected_sha256", "actual_sha256"}
                    and refusal["schema_version"] == 1
                    and refusal["error"] in {"SOURCE_PUBLISHED_MANIFEST_MISMATCH", "SOURCE_STAGED_MANIFEST_MISMATCH"}
                    and all(isinstance(refusal[name], str) and len(refusal[name]) == 64
                            and all(character in "0123456789abcdef" for character in refusal[name])
                            for name in ("expected_sha256", "actual_sha256"))):
                details.update({"receiver_error": refusal["error"], "expected_sha256": refusal["expected_sha256"],
                                "actual_sha256": refusal["actual_sha256"]})
        except (ValueError, UnicodeError):
            pass
        raise DevctlError(code, "SOURCE_TRANSFER_FAILED", "Pinned guest source receiver failed verification", details=details)
    try:
        result = contract.decode(output)
    except (ValueError, UnicodeError) as error:
        raise failure(ExitCode.VERIFICATION_FAILURE, "INVALID_SOURCE_RECEIPT", "Invalid source publication receipt") from error
    fields = {"schema_version", "snapshot_digest", "guest_digest", "guest_source_path", "file_count", "reused", "identity"}
    if not isinstance(result, dict) or set(result) != fields or type(result["schema_version"]) is not int or result["schema_version"] != 1 or type(result["reused"]) is not bool or type(result["file_count"]) is not int or result["snapshot_digest"] != request["digest"] or result["guest_digest"] != request["digest"] or result["identity"] != request["identity"] or result["file_count"] != len(request["manifest"]["files"]) or result["guest_source_path"] != request["source_root"] + "/" + request["digest"]:
        raise failure(ExitCode.VERIFICATION_FAILURE, "SOURCE_RECEIPT_MISMATCH", "Guest receipt does not match the requested snapshot and target")
    return result


def synchronize(config):
    trust, identity = guest.enrolled_identity(config)
    if identity["guest_role"] not in ("development", "acceptance"):
        raise failure(ExitCode.TARGET_MISMATCH, "SOURCE_ROLE_MISMATCH", "Source publication requires a development or acceptance guest")
    manifest, digest, contents = collect(config.root)
    request = {"schema_version": 1, "operation": "publish-source", "identity": identity,
               "source_root": config.values["guest_source_root"], "manifest": manifest, "digest": digest}
    result = transfer(config, request, contents)
    directory = private_directory(config.root, ".local/snapshots")
    path = directory / (digest + ".json")
    # Store local source provenance separately from per-operation guest evidence.
    if not path.exists():
        try:
            write_new(path, contract.canonical({"schema_version": 1, "snapshot_digest": digest, "manifest": manifest}))
        except FileExistsError:
            pass
    guest.private_file(path)
    expected_record = {"schema_version": 1, "snapshot_digest": digest, "manifest": manifest}
    if path.stat().st_size > contract.MAX_HEADER + 4096 or contract.decode(path.read_bytes()) != expected_record:
        raise failure(ExitCode.VERIFICATION_FAILURE, "LOCAL_SOURCE_RECORD_MISMATCH", "Local source provenance differs from the verified snapshot")
    return ExitCode.SUCCESS, {**result, "identity_verified": True, "git_head": manifest["git_head"], "dirty": manifest["dirty"],
                              "host_key_fingerprint": trust["host_key_fingerprint"], "release_digest": digest,
                              "artifact_path": str(path), "flake_reference": "path:" + result["guest_source_path"]}
