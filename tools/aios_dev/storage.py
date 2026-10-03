"""Read-only Btrfs identity and explicit offline legacy storage enrollment."""
import errno
import fcntl
import json
import os
import stat
import struct
import uuid

from .config import invalid, project_path
from .errors import ExitCode
from .provision import failure, operation_lock, private_directory, write_json_new

# Linux UAPI linux/btrfs.h: _IOR(0x94, 31, fs_info_args[1024]) and
# _IOR(0x94, 60, get_subvol_info_args[504]). Neither ioctl changes the filesystem.
FS_INFO = 0x8400941F
SUBVOL_INFO = 0x81F8943C


def validate_identity(identity):
    fields = {"schema_version", "filesystem", "filesystem_uuid", "subvolume_id", "subvolume_uuid"}
    if (not isinstance(identity, dict) or set(identity) != fields or
        type(identity["schema_version"]) is not int or identity["schema_version"] != 1 or
        identity["filesystem"] != "btrfs" or type(identity["subvolume_id"]) is not int or
        identity["subvolume_id"] < 5):
        raise invalid("Invalid persistent Btrfs identity")
    for field in ("filesystem_uuid", "subvolume_uuid"):
        try:
            if str(uuid.UUID(identity[field])) != identity[field]:
                raise ValueError
        except (ValueError, TypeError, AttributeError):
            raise invalid("Invalid persistent Btrfs UUID")
    if identity["filesystem_uuid"] == str(uuid.UUID(int=0)):
        raise invalid("Invalid native Btrfs filesystem UUID")


def observe(path):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid():
            raise failure(ExitCode.TARGET_MISMATCH, "TARGET_MISMATCH", "Virtual disk type or owner changed")
        filesystem, subvolume = bytearray(1024), bytearray(504)
        try:
            fcntl.ioctl(descriptor, FS_INFO, filesystem, True)
        except OSError as error:
            if error.errno in (errno.ENOTTY, errno.EOPNOTSUPP):
                current = path.lstat()
                if (current.st_dev, current.st_ino) != (info.st_dev, info.st_ino):
                    raise failure(ExitCode.TARGET_MISMATCH, "TARGET_MISMATCH", "Virtual disk changed during storage observation")
                return info, None
            raise
        fcntl.ioctl(descriptor, SUBVOL_INFO, subvolume, True)
        identity = {"schema_version": 1, "filesystem": "btrfs",
                    "filesystem_uuid": str(uuid.UUID(bytes=bytes(filesystem[16:32]))),
                    "subvolume_id": struct.unpack_from("=Q", subvolume)[0],
                    "subvolume_uuid": str(uuid.UUID(bytes=bytes(subvolume[296:312])))}
        validate_identity(identity)
        current = path.lstat()
        if (current.st_dev, current.st_ino) != (info.st_dev, info.st_ino):
            raise failure(ExitCode.TARGET_MISMATCH, "TARGET_MISMATCH", "Virtual disk changed during storage observation")
        return info, identity
    finally:
        os.close(descriptor)


def verify(path, record):
    info, identity = observe(path)
    if "storage_identity" in record:
        validate_identity(record["storage_identity"])
        same_filesystem = identity is not None and identity == record["storage_identity"]
    else:
        same_filesystem = info.st_dev == record.get("disk_device")
    if info.st_ino != record.get("disk_inode") or not same_filesystem:
        raise failure(ExitCode.TARGET_MISMATCH, "TARGET_MISMATCH", "Virtual disk identity changed")
    return info


def rebind(config, previous_device, previous_inode, filesystem_uuid, subvolume_uuid):
    """The operator acknowledges the exact old disk and current Btrfs identities.

    This only enrolls persistent host storage metadata. It never starts a VM,
    changes disk bytes, replaces SSH trust, or accepts a new guest identity.
    """
    from . import vm
    if config.values["provider"] != "qemu":
        raise failure(ExitCode.UNSUPPORTED_CAPABILITY, "UNSUPPORTED_CAPABILITY", "Storage enrollment requires an owned QEMU disk")
    for value in (previous_device, previous_inode):
        if type(value) is not int or value < 0:
            raise invalid("Previous device and inode must be nonnegative integers")
    for value in (filesystem_uuid, subvolume_uuid):
        try:
            if str(uuid.UUID(value)) != value:
                raise ValueError
        except (ValueError, TypeError, AttributeError):
            raise invalid("Storage UUID acknowledgment must be canonical")
    with operation_lock(config.root):
        if any(path.exists() or path.is_symlink() for path in (config.root / ".local/vm/process.json",
                                         config.root / ".local/vm/qemu.pid",
                                         config.paths["qmp_socket"], config.paths["serial_socket"])):
            raise failure(ExitCode.AUTHORIZATION_NEEDED, "VM_NOT_STOPPED", "Storage enrollment requires a stopped VM without retained control sockets")
        # All original configuration, authorization and media checks still run.
        # Only this explicitly acknowledged offline command can reach material
        # validation without the old transient device number matching.
        record = vm._load_record_material(config)
        if (previous_device, previous_inode) != (record["disk_device"], record["disk_inode"]):
            raise failure(ExitCode.TARGET_MISMATCH, "TARGET_MISMATCH", "Previous storage acknowledgment differs from the provisioning record")
        info, identity = observe(config.paths["disk_image"])
        if (info.st_ino != previous_inode or identity is None or
            identity["filesystem_uuid"] != filesystem_uuid or identity["subvolume_uuid"] != subvolume_uuid):
            raise failure(ExitCode.TARGET_MISMATCH, "TARGET_MISMATCH", "Disk inode or acknowledged Btrfs identity differs")
        if "storage_identity" in record:
            verify(config.paths["disk_image"], record)
            return ExitCode.SUCCESS, {"state": "storage-already-enrolled", "storage_identity": identity,
                                      "guest_identity_verified": False}
        path = project_path(config.root, ".local/provisioning.json", ".local")
        original_info, original = path.lstat(), path.read_bytes()
        if (not stat.S_ISREG(original_info.st_mode) or original_info.st_uid != os.getuid() or
            stat.S_IMODE(original_info.st_mode) != 0o600 or json.loads(original) != record):
            raise invalid("Provisioning record changed or is not private")
        operation = str(uuid.uuid4())
        directory = private_directory(config.root, ".local/storage-enrollments/" + operation)
        write_json_new(directory / "previous-provisioning.json", record)
        updated = {**record, "storage_identity": identity}
        staged = directory / "provisioning.json"
        write_json_new(staged, updated)
        write_json_new(directory / "receipt.json", {
            "schema_version": 1, "operation": "acknowledged-offline-storage-enrollment",
            "previous_device": previous_device, "previous_inode": previous_inode,
            "observed_device": info.st_dev, "storage_identity": identity,
            "guest_uuid": record["plan"]["guest_uuid"],
            "installation_uuid": record["plan"]["installation_uuid"],
            "guest_identity_verified": False, "disk_bytes_changed": False,
        })
        verify(config.paths["disk_image"], updated)
        current = path.lstat()
        if ((current.st_dev, current.st_ino) != (original_info.st_dev, original_info.st_ino) or
            path.read_bytes() != original):
            raise failure(ExitCode.TARGET_MISMATCH, "TARGET_MISMATCH", "Provisioning record changed before storage enrollment")
        os.replace(staged, path)
        descriptor = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
        return ExitCode.SUCCESS, {"state": "storage-enrolled", "storage_identity": identity,
                                  "artifact_path": str(directory / "receipt.json"),
                                  "guest_identity_verified": False}
