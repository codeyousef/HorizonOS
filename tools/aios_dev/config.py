"""Strict target configuration and project-owned path resolution; no writes."""
import ipaddress
import json
import re
import stat
import uuid
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

from .errors import DevctlError, ExitCode

RUNTIME_FIELDS = ("disk_image", "nvram_file", "qmp_socket", "serial_socket", "serial_log")
SSH_FIELDS = ("identity_file", "known_hosts_file")
REQUIRED_FIELDS = {
    "schema_version", "name", "transport", "provider", "ssh_host", "ssh_port",
    "ssh_user", "guest_source_root", "guest_build_target", "vcpus", "memory_mib",
    "disk_gib", *RUNTIME_FIELDS, *SSH_FIELDS,
}
OPTIONAL_FIELDS = {"guest_uuid", "installation_uuid", "guest_role"}
NAME_PATTERN = re.compile(r"[a-zA-Z0-9][a-zA-Z0-9_.-]{0,63}\Z")
USER_PATTERN = re.compile(r"[a-z_][a-z0-9_-]{0,31}\Z")


def invalid(message: str) -> DevctlError:
    return DevctlError(ExitCode.INVALID_INPUT, "INVALID_ARGUMENT", message)


def object_without_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise invalid(f"Duplicate configuration field: {key}")
        result[key] = value
    return result


def read_json(path: Path):
    try:
        info = path.stat()
        if not stat.S_ISREG(info.st_mode):
            raise invalid("Configuration must be a regular file")
        if info.st_size > 65_536:
            raise invalid("Configuration exceeds 64 KiB")
        contents = path.read_bytes()
        if len(contents) > 65_536:
            raise invalid("Configuration exceeds 64 KiB")
        return json.loads(
            contents.decode("utf-8"),
            object_pairs_hook=object_without_duplicates,
            parse_constant=lambda _: (_ for _ in ()).throw(invalid("Non-finite JSON number")),
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise invalid(f"Cannot read configuration {path.name}: {type(error).__name__}") from error


def project_path(root: Path, value: str, allowed_directory: str) -> Path:
    if not isinstance(value, str) or not value or "\x00" in value:
        raise invalid("Project paths must be non-empty relative strings")
    relative = PurePosixPath(value)
    if relative.is_absolute() or ".." in relative.parts:
        raise invalid("Absolute paths and traversal are forbidden in project paths")
    root = root.resolve(strict=True)
    allowed = root / allowed_directory
    lexical = root / relative
    if not lexical.is_relative_to(allowed) or lexical == allowed:
        raise invalid(f"Path must be a child of {allowed_directory}/")
    try:
        resolved = lexical.resolve(strict=False)
        allowed_resolved = allowed.resolve(strict=False)
    except (OSError, RuntimeError) as error:
        raise invalid("Cannot safely resolve project path") from error
    if allowed_resolved != allowed or not resolved.is_relative_to(allowed):
        raise invalid(f"Symlink escape from {allowed_directory}/")
    if resolved == allowed_resolved:
        raise invalid("A file path cannot resolve to its containing directory")
    return resolved


@dataclass(frozen=True)
class VMConfig:
    root: Path
    values: dict
    paths: dict[str, Path]
    configured: bool

    @classmethod
    def from_data(cls, root: Path, data, *, configured: bool = False):
        if not isinstance(data, dict):
            raise invalid("VM configuration must be a JSON object")
        missing = REQUIRED_FIELDS - data.keys()
        unknown = data.keys() - REQUIRED_FIELDS - OPTIONAL_FIELDS
        if missing or unknown:
            raise invalid(f"Configuration fields: missing={sorted(missing)}, unknown={sorted(unknown)}")
        for key, low, high in (
            ("schema_version", 1, 1), ("ssh_port", 1, 65535), ("vcpus", 1, 1024),
            ("memory_mib", 512, 1_048_576), ("disk_gib", 8, 1_048_576),
        ):
            value = data[key]
            if type(value) is not int or not low <= value <= high:
                raise invalid(f"{key} must be an integer in {low}..{high}")
        for key in ("name", "guest_build_target"):
            if not isinstance(data[key], str) or not NAME_PATTERN.fullmatch(data[key]):
                raise invalid(f"Invalid {key}")
        if data["transport"] != "ssh" or data["provider"] not in ("qemu", "external"):
            raise invalid("Only SSH with qemu or external providers is supported")
        if not isinstance(data["ssh_user"], str) or not USER_PATTERN.fullmatch(data["ssh_user"]):
            raise invalid("Invalid SSH user")
        host = data["ssh_host"]
        if not isinstance(host, str) or not host or len(host) > 253:
            raise invalid("Invalid SSH host")
        try:
            address = ipaddress.ip_address(host)
        except ValueError:
            address = None
            if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9.-]*", host):
                raise invalid("Invalid SSH hostname")
        if data["provider"] == "qemu" and (address is None or not address.is_loopback):
            raise invalid("QEMU SSH forwarding must use a literal loopback address")
        source_root = data["guest_source_root"]
        if not isinstance(source_root, str) or "\x00" in source_root:
            raise invalid("Invalid guest source root")
        guest_path = PurePosixPath(source_root)
        expected_home = PurePosixPath("/home") / data["ssh_user"]
        if ".." in guest_path.parts or not guest_path.is_relative_to(expected_home) or guest_path == expected_home:
            raise invalid("Guest source root must be below the configured user's /home directory")
        for key in ("guest_uuid", "installation_uuid"):
            if key in data:
                try:
                    value = data[key]
                    if not isinstance(value, str) or str(uuid.UUID(value)) != value:
                        raise ValueError
                except ValueError as error:
                    raise invalid(f"{key} must be a canonical UUID") from error
        if data.get("guest_role", "development") not in ("development", "acceptance", "recovery"):
            raise invalid("Unknown guest role")
        paths = {key: project_path(root, data[key], ".local/vm") for key in RUNTIME_FIELDS}
        paths.update({key: project_path(root, data[key], ".local/ssh") for key in SSH_FIELDS})
        if len(set(paths.values())) != len(paths):
            raise invalid("Runtime and SSH paths must identify distinct files")
        for key, path in paths.items():
            if path.exists():
                mode = path.stat().st_mode
                expected = stat.S_ISSOCK if key in ("qmp_socket", "serial_socket") else stat.S_ISREG
                if not expected(mode):
                    raise invalid(f"{key} has an unexpected file type")
        return cls(root.resolve(strict=True), dict(data), paths, configured)


def load_config(root: Path) -> VMConfig:
    root = root.resolve(strict=True)
    local = root / ".local/vm.json"
    # Reject a .local symlink escape even when loading a local configuration itself.
    local = project_path(root, ".local/vm.json", ".local")
    configured = local.exists()
    source = local if configured else root / "dev/vm.example.json"
    return VMConfig.from_data(root, read_json(source), configured=configured)
