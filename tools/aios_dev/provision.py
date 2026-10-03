"""Prepare public bootstrap media and a fresh project-owned virtual disk.

This module never installs an OS remotely or operates on host block devices.
The console installer performs its own identity/disk checks before partitioning.
"""
import hashlib
from contextlib import contextmanager
import json
import os
import fcntl
from pathlib import Path
import re
import shutil
import stat
import subprocess
import urllib.parse
import urllib.request
import uuid

from .config import VMConfig, invalid, project_path, read_json
from .doctor import host_report
from .errors import DevctlError, ExitCode

CHECKSUM_URL = "https://channels.nixos.org/nixos-26.05/latest-nixos-minimal-x86_64-linux.iso.sha256"
OFFICIAL_HOSTS = {"channels.nixos.org", "releases.nixos.org"}
DISK_SERIAL = "AIOS_DEV_ROOT"
SOURCE_ROOTS = {"crates", "native", "nix", "schemas", "capabilities", "policies", "models", "prompts", "tools", "dev", "tests", "docs"}
SOURCE_FILES = {".gitignore", "AGENTS.md", "README.md", "flake.nix", "flake.lock", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "VERSION", "CHANGELOG.md"}
PRIVATE_PARTS = {".git", ".local", ".agents", ".aws", ".codex", ".ssh", ".gnupg", ".venv", "__pycache__", "target", "node_modules"}
PRIVATE_SUFFIXES = {".key", ".pem", ".qcow2", ".iso", ".gguf", ".safetensors", ".pyc"}


def failure(code, label, message):
    return DevctlError(code, label, message)


def private_directory(root: Path, relative: str) -> Path:
    # Validate canonical paths before creating any component, never chmod a link target.
    target = project_path(root, relative + "/probe", relative).parent
    current = root
    for part in Path(relative).parts:
        current = current / part
        if current.is_symlink():
            raise invalid("Private directories must not be symlinks")
        if not current.exists():
            current.mkdir(mode=0o700)
        info = current.stat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or stat.S_IMODE(info.st_mode) & 0o077:
            raise invalid(f"Private directory must be owned by this user with mode 0700: {current.name}")
    return target


def write_new(path: Path, contents: bytes, mode=0o600):
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode), "wb") as handle:
        handle.write(contents)
        handle.flush()
        os.fsync(handle.fileno())


def write_json_new(path: Path, data):
    write_new(path, (json.dumps(data, indent=2, sort_keys=True) + "\n").encode())


def digest_file(path: Path) -> str:
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def run(arguments, timeout=60):
    try:
        result = subprocess.run(arguments, capture_output=True, text=True, check=False, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        raise failure(ExitCode.TIMEOUT, "TIMEOUT", "Bootstrap tool timed out") from error
    if result.returncode:
        # Never include SSH key contents or arbitrary full subprocess output.
        raise DevctlError(ExitCode.OPERATION_FAILURE, "OPERATION_FAILED", f"{Path(arguments[0]).name} exited {result.returncode}", details={"upstream_exit": result.returncode})
    return result


def prepare_plan(config: VMConfig) -> dict:
    if config.values["provider"] != "qemu":
        raise failure(ExitCode.UNSUPPORTED_CAPABILITY, "UNSUPPORTED_CAPABILITY", "External targets cannot be provisioned by QEMU")
    if config.values["ssh_user"] != "dev" or config.values["guest_build_target"] not in ("aios-dev", "aios-desktop-test") or config.values.get("guest_role", "development") != "development":
        raise invalid("Bootstrap requires the dev user, development role and a registered image target")
    private_directory(config.root, ".local")
    plan_path = project_path(config.root, ".local/provisioning-plan.json", ".local")
    if plan_path.exists():
        plan = read_json(plan_path)
        validate_plan(config, plan)
        return plan
    if config.paths["disk_image"].exists() or config.paths["nvram_file"].exists():
        raise failure(ExitCode.AUTHORIZATION_NEEDED, "REINSTALL_DENIED", "Existing disk/NVRAM cannot be provisioned; reinstall support requires separate destructive authorization")
    plan = {
        "schema_version": 1, "guest_uuid": config.values.get("guest_uuid", str(uuid.uuid4())),
        "installation_uuid": config.values.get("installation_uuid", str(uuid.uuid4())),
        "guest_role": "development", "disk_serial": DISK_SERIAL,
        "configuration": config.values, "disk_image": str(config.paths["disk_image"]),
        "checksum_url": CHECKSUM_URL, "operation": "provision-fresh-virtual-disk",
    }
    validate_plan(config, plan)
    write_json_new(plan_path, plan)
    return plan


def select_fresh_defaults(config: VMConfig) -> VMConfig:
    """Freeze measured defaults once; explicit or existing VM plans stay exact."""
    if config.configured or config.values["provider"] != "qemu" or any(
        path.exists() for path in (config.root / ".local/provisioning-plan.json",
                                  config.root / ".local/provisioning.json",
                                  config.paths["disk_image"], config.paths["nvram_file"])
    ):
        return config
    from .resources import recommend
    observations = host_report(config)
    selection = recommend(observations)
    values = {**config.values, **{key: selection[key] for key in ("vcpus", "memory_mib", "disk_gib")}}
    selected = VMConfig.from_data(config.root, values, configured=True)
    private_directory(config.root, ".local")
    # Freeze configuration before issuing a provisioning UUID. Subsequent calls
    # use exactly this configuration rather than recomputing from changing load.
    write_json_new(config.root / ".local/vm.json", values)
    write_json_new(config.root / ".local/provisioning-resources.json", {
        "schema_version": 1, "configuration": values, "selection": selection,
        "observations": {key: observations[key] for key in ("logical_cpus", "memory", "disk")},
        "purpose": "fresh-development-defaults-not-inference-minimums",
    })
    return selected


def validate_plan(config: VMConfig, plan):
    if not isinstance(plan, dict) or plan.get("schema_version") != 1 or plan.get("configuration") != config.values or plan.get("disk_image") != str(config.paths["disk_image"]) or plan.get("disk_serial") != DISK_SERIAL or plan.get("guest_role") != "development" or plan.get("checksum_url") != CHECKSUM_URL or plan.get("operation") != "provision-fresh-virtual-disk":
        raise failure(ExitCode.TARGET_MISMATCH, "TARGET_MISMATCH", "Provisioning plan no longer matches the exact VM configuration")
    for field in ("guest_uuid", "installation_uuid"):
        value = plan.get(field)
        try:
            valid = isinstance(value, str) and str(uuid.UUID(value)) == value
        except ValueError:
            valid = False
        if not valid:
            raise invalid("Invalid provisioning UUID")


def official_url(url: str) -> str:
    parsed = urllib.parse.urlsplit(url)
    if parsed.scheme != "https" or parsed.hostname not in OFFICIAL_HOSTS or parsed.port not in (None, 443) or parsed.username or parsed.password or parsed.query or parsed.fragment:
        raise failure(ExitCode.VERIFICATION_FAILURE, "MEDIA_PROVENANCE_FAILED", "Installer media must use an official NixOS HTTPS release URL")
    return url


class OfficialRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        official_url(newurl)
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def fetch_media(directory: Path) -> dict:
    opener = urllib.request.build_opener(OfficialRedirects())
    try:
        with opener.open(official_url(CHECKSUM_URL), timeout=30) as response:
            checksum_url = official_url(response.geturl())
            content = response.read(4097)
        if len(content) > 4096 or not checksum_url.endswith(".iso.sha256"):
            raise ValueError("Unexpected checksum document")
        words = content.decode("ascii").strip().split()
        if not words or not re.fullmatch(r"[0-9a-f]{64}", words[0]):
            raise ValueError("Invalid checksum")
        expected = words[0]
        url = official_url(checksum_url.removesuffix(".sha256"))
        filename = urllib.parse.urlsplit(url).path.rsplit("/", 1)[-1]
        if not re.fullmatch(r"nixos-minimal-26\.05\.[A-Za-z0-9.-]+-x86_64-linux\.iso", filename):
            raise ValueError("Unexpected release filename")
        path = directory / filename
        if path.exists():
            if path.is_symlink() or not path.is_file() or digest_file(path) != expected:
                raise failure(ExitCode.VERIFICATION_FAILURE, "MEDIA_DIGEST_MISMATCH", "Cached installer failed checksum verification")
        else:
            partial = directory / (filename + ".part")
            with opener.open(url, timeout=30) as response:
                if official_url(response.geturl()) != url:
                    raise ValueError("Unexpected ISO redirect")
                with os.fdopen(os.open(partial, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600), "wb") as handle:
                    size = 0
                    while chunk := response.read(1024 * 1024):
                        size += len(chunk)
                        if size > 4 * 1024**3:
                            raise ValueError("Installer exceeds size limit")
                        handle.write(chunk)
            if digest_file(partial) != expected:
                partial.unlink()
                raise failure(ExitCode.VERIFICATION_FAILURE, "MEDIA_DIGEST_MISMATCH", "Downloaded installer failed official checksum")
            partial.rename(path)
        return {"url": url, "checksum_requested_url": CHECKSUM_URL, "checksum_provenance_url": checksum_url, "sha256": expected, "verified": True, "path": str(path)}
    except TimeoutError as error:
        raise failure(ExitCode.TIMEOUT, "MEDIA_DOWNLOAD_TIMEOUT", "Official installer retrieval timed out") from error
    except (OSError, UnicodeError, ValueError) as error:
        raise failure(ExitCode.OPERATION_FAILURE, "MEDIA_DOWNLOAD_FAILED", f"Official installer retrieval failed: {type(error).__name__}") from error


def source_files(root: Path) -> list[Path]:
    result = run(["git", "-C", str(root), "ls-files", "-z"], timeout=10)
    files = []
    for name in result.stdout.split("\0"):
        if not name:
            continue
        relative = Path(name)
        if name == "nix/machines/aios-dev/enrollment.json":
            raise invalid("Bootstrap enrollment is generated only from the verified fresh target and public key")
        if relative.is_absolute() or ".." in relative.parts or any(p in PRIVATE_PARTS or p.startswith(".env") for p in relative.parts) or relative.suffix in PRIVATE_SUFFIXES:
            raise invalid("Tracked source contains a forbidden/private path")
        if relative.parts[0] not in SOURCE_ROOTS and name not in SOURCE_FILES:
            raise invalid(f"Unreviewed seed source domain: {relative.parts[0]}")
        path = root / relative
        if path.is_symlink() or not path.is_file() or path.stat().st_size > 16 * 1024**2:
            raise invalid("Seed source must consist of bounded regular files without symlinks")
        contents = path.read_bytes()
        if re.search(rb"-----BEGIN (?:[A-Z]+ )?PRIVATE KEY-----", contents):
            raise invalid("Private key material cannot enter bootstrap source")
        files.append(relative)
    if not files or Path("flake.nix") not in files or Path("dev/seed/bootstrap.sh") not in files:
        raise invalid("Bootstrap source must be tracked and include its flake and installer")
    return sorted(files)


def seed_manifest(directory: Path):
    lines = []
    for path in sorted(directory.rglob("*")):
        if path.is_file():
            name = path.relative_to(directory).as_posix()
            if "\n" in name or "\\" in name:
                raise invalid("Seed filenames cannot contain newline or backslash")
            lines.append(f"{digest_file(path)}  {name}\n")
    write_new(directory / "manifest.sha256", "".join(lines).encode(), 0o644)


def refresh_seed(config: VMConfig):
    """Replace public bootstrap media only, while the managed VM is stopped."""
    from .vm import load_record
    with operation_lock(config.root):
        record = load_record(config)
        controls = [config.root / ".local/vm/process.json", config.root / ".local/vm/qemu.pid",
                    config.paths["qmp_socket"], config.paths["serial_socket"]]
        if any(path.exists() for path in controls):
            raise failure(ExitCode.TARGET_MISMATCH, "VM_MUST_BE_STOPPED", "Stop the verified VM before refreshing public bootstrap media")
        directory = private_directory(config.root, ".local/vm")
        seed = private_directory(config.root, f".local/vm/seed-{uuid.uuid4()}")
        public = Path(str(config.paths["identity_file"]) + ".pub").read_text().strip()
        if not re.fullmatch(r"ssh-ed25519 [A-Za-z0-9+/=]+(?: [a-zA-Z0-9_-]+)?", public):
            raise invalid("Unexpected public key format")
        for relative in source_files(config.root):
            target = seed / "source" / relative
            target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
            write_new(target, (config.root / relative).read_bytes(), 0o644)
        plan = record["plan"]
        for name, value in (("guest.uuid", plan["guest_uuid"]), ("installation.uuid", plan["installation_uuid"]),
                            ("disk.serial", DISK_SERIAL), ("authorized.uuid", record["authorized_uuid"]), ("dev.pub", public), ("image.target", config.values["guest_build_target"])):
            write_new(seed / name, (value + "\n").encode(), 0o644)
        write_new(seed / "bootstrap.sh", (config.root / "dev/seed/bootstrap.sh").read_bytes(), 0o644)
        seed_manifest(seed)
        iso = directory / (seed.name + ".iso")
        run(["xorriso", "-as", "mkisofs", "-quiet", "-V", "AIOS_SEED", "-o", str(iso), str(seed)])
        previous = config.root / ".local/provisioning.json"
        write_new(directory / (seed.name + "-previous-provisioning.json"), previous.read_bytes())
        source = run(["git", "-C", str(config.root), "rev-parse", "HEAD"]).stdout.strip()
        dirty = bool(run(["git", "-C", str(config.root), "status", "--porcelain"]).stdout.strip())
        updated = {**record, "seed_iso": str(iso), "seed_sha256": digest_file(iso), "seed_source_commit": source, "seed_source_dirty": dirty}
        temporary = directory / (seed.name + "-provisioning.json")
        write_json_new(temporary, updated)
        os.replace(temporary, previous)
        return ExitCode.SUCCESS, {"seed_iso": str(iso), "seed_sha256": updated["seed_sha256"], "source_commit": source, "source_dirty": dirty}


@contextmanager
def operation_lock(root: Path):
    private_directory(root, ".local")
    lock = root / ".local/provisioning.lock"
    with os.fdopen(os.open(lock, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600), "r+") as handle:
        info = os.fstat(handle.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_nlink != 1 or stat.S_IMODE(info.st_mode) & 0o077:
            raise invalid("VM operation lock must be a private regular file")
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise failure(ExitCode.OPERATION_FAILURE, "VM_OPERATION_BUSY", "Another VM operation is active") from error
        yield


def create(config: VMConfig, authorization: str | None, *, size_defaults=False) -> tuple[ExitCode, dict]:
    if config.values["provider"] != "qemu":
        raise failure(ExitCode.UNSUPPORTED_CAPABILITY, "UNSUPPORTED_CAPABILITY", "External targets cannot be provisioned by QEMU")
    with operation_lock(config.root):
        if size_defaults:
            config = select_fresh_defaults(config)
        return create_locked(config, authorization)


def create_locked(config: VMConfig, authorization: str | None) -> tuple[ExitCode, dict]:
    plan = prepare_plan(config)
    if authorization is None:
        return ExitCode.AUTHORIZATION_NEEDED, {"plan": plan, "message": "Review the fresh virtual disk plan; pass --authorize-provision with this guest UUID to prepare it. No disk has been created."}
    if authorization != plan["guest_uuid"]:
        raise failure(ExitCode.TARGET_MISMATCH, "TARGET_MISMATCH", "Provisioning authorization does not match the planned VM UUID")
    if config.paths["disk_image"].exists() or config.paths["nvram_file"].exists() or (config.root / ".local/provisioning.json").exists():
        raise failure(ExitCode.AUTHORIZATION_NEEDED, "REINSTALL_DENIED", "Refusing to overwrite existing VM artifacts")
    report = host_report(config)
    if report["missing_prerequisites"]:
        return ExitCode.UNMET_PREREQUISITE, report
    if shutil.which("ssh-keygen") is None:
        raise failure(ExitCode.UNMET_PREREQUISITE, "MISSING_TOOL", "ssh-keygen is required for the dedicated development key")
    vm_dir = private_directory(config.root, ".local/vm")
    private_directory(config.root, ".local/ssh")
    for path in config.paths.values():
        private_directory(config.root, str(path.parent.relative_to(config.root)))
    sources = source_files(config.root)
    media = fetch_media(vm_dir)
    key = config.paths["identity_file"]
    if key.exists() or Path(str(key) + ".pub").exists():
        raise failure(ExitCode.AUTHORIZATION_NEEDED, "KEY_REUSE_DENIED", "Bootstrap requires a fresh dedicated development key")
    run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-C", "aios-development", "-f", str(key)])
    key.chmod(0o600)
    public = Path(str(key) + ".pub").read_text().strip()
    if not re.fullmatch(r"ssh-ed25519 [A-Za-z0-9+/=]+(?: [a-zA-Z0-9_-]+)?", public):
        raise invalid("Unexpected public key format")
    seed = vm_dir / "seed"
    seed.mkdir(mode=0o700)
    for relative in sources:
        target = seed / "source" / relative
        target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        write_new(target, (config.root / relative).read_bytes(), 0o644)
    for name, value in (("guest.uuid", plan["guest_uuid"]), ("installation.uuid", plan["installation_uuid"]), ("disk.serial", DISK_SERIAL), ("authorized.uuid", authorization), ("dev.pub", public), ("image.target", config.values["guest_build_target"])):
        write_new(seed / name, (value + "\n").encode(), 0o644)
    write_new(seed / "bootstrap.sh", (config.root / "dev/seed/bootstrap.sh").read_bytes(), 0o644)
    seed_manifest(seed)
    seed_iso = vm_dir / "seed.iso"
    run(["xorriso", "-as", "mkisofs", "-quiet", "-V", "AIOS_SEED", "-o", str(seed_iso), str(seed)])
    run(["qemu-img", "create", "-f", "qcow2", str(config.paths["disk_image"]), f"{config.values['disk_gib']}G"])
    config.paths["disk_image"].chmod(0o600)
    write_new(config.paths["nvram_file"], Path(report["firmware"]["variables_template"]).read_bytes())
    runtime = {**config.values, "guest_uuid": plan["guest_uuid"], "installation_uuid": plan["installation_uuid"], "guest_role": "development"}
    # An existing caller-provided local configuration is preserved; identity lives
    # in provisioning.json until the enrollment workflow installs trusted fields.
    record = {"schema_version": 1, "state": "prepared", "plan": plan, "runtime_configuration": runtime, "media": media,
              "seed_iso": str(seed_iso), "seed_sha256": digest_file(seed_iso), "firmware_code": report["firmware"]["code"],
              "firmware_sha256": digest_file(Path(report["firmware"]["code"])), "disk_device": config.paths["disk_image"].stat().st_dev,
              "disk_inode": config.paths["disk_image"].stat().st_ino, "authorized_operation": "provision-fresh-virtual-disk", "authorized_uuid": authorization}
    write_json_new(config.root / ".local/provisioning.json", record)
    return ExitCode.SUCCESS, {"guest_uuid": plan["guest_uuid"], "installation_uuid": plan["installation_uuid"], "state": "prepared", "media": media, "seed_sha256": record["seed_sha256"], "artifact_path": str(config.root / ".local/provisioning.json")}
