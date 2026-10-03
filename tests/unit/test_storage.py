"""Host-only storage fixtures; no VM or guest identity is certified."""
import errno
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch
import uuid

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from aios_dev import storage, vm
from aios_dev.cli import parser
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError, ExitCode
from aios_dev.provision import digest_file, prepare_plan, private_directory, write_json_new

IDENTITY = {"schema_version": 1, "filesystem": "btrfs",
            "filesystem_uuid": "38a24835-0bcb-4215-bbd3-6951873512a5",
            "subvolume_id": 5, "subvolume_uuid": "5e6da7c0-5cd3-4895-8c5d-85ee60dce8ee"}


class StorageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="aios-storage-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        data = json.loads((ROOT / "dev/vm.example.json").read_text())
        self.config = VMConfig.from_data(self.root, data)
        plan = prepare_plan(self.config)
        directory = private_directory(self.root, ".local/vm")
        self.disk = self.config.paths["disk_image"]
        self.disk.write_bytes(b"unchanged disk fixture")
        self.disk.chmod(0o600)
        self.info = self.disk.stat()
        seed, media, firmware = (directory / name for name in ("seed.iso", "installer.iso", "firmware.fd"))
        for path in (seed, media, firmware):
            path.write_bytes(b"fixed media fixture")
        self.record = {"schema_version": 1, "state": "prepared", "plan": plan,
                       "authorized_uuid": plan["guest_uuid"], "authorized_operation": "provision-fresh-virtual-disk",
                       "disk_inode": self.info.st_ino, "disk_device": self.info.st_dev + 1,
                       "seed_iso": str(seed), "seed_sha256": digest_file(seed),
                       "firmware_code": str(firmware), "firmware_sha256": digest_file(firmware),
                       "media": {"path": str(media), "sha256": digest_file(media)}}
        self.path = self.root / ".local/provisioning.json"
        write_json_new(self.path, self.record)

    def invoke(self, **changes):
        args = {"previous_device": self.record["disk_device"], "previous_inode": self.record["disk_inode"],
                "filesystem_uuid": IDENTITY["filesystem_uuid"], "subvolume_uuid": IDENTITY["subvolume_uuid"]}
        return storage.rebind(self.config, **{**args, **changes})

    def test_legacy_device_mismatch_still_stops_without_automatic_enrollment(self):
        original = self.path.read_bytes()
        with patch.object(storage, "observe", return_value=(self.info, IDENTITY)):
            with self.assertRaises(DevctlError) as caught:
                vm.load_record(self.config)
        self.assertEqual(caught.exception.code, "TARGET_MISMATCH")
        self.assertEqual(self.path.read_bytes(), original)
        self.assertFalse((self.root / ".local/storage-enrollments").exists())

    def test_acknowledged_enrollment_preserves_disk_and_trust_then_is_idempotent(self):
        original = self.path.read_bytes()
        before = self.disk.read_bytes()
        with patch.object(storage, "observe", return_value=(self.info, IDENTITY)):
            code, result = self.invoke()
            bound = vm.load_record(self.config)
            again_code, again = self.invoke()
        self.assertEqual(code, ExitCode.SUCCESS)
        self.assertEqual(again_code, ExitCode.SUCCESS)
        self.assertEqual(again["state"], "storage-already-enrolled")
        self.assertFalse(result["guest_identity_verified"])
        self.assertEqual(bound, {**self.record, "storage_identity": IDENTITY})
        self.assertEqual(self.disk.read_bytes(), before)
        self.assertFalse((self.root / ".local/ssh").exists())
        self.assertEqual(self.path.stat().st_mode & 0o777, 0o600)
        backups = list((self.root / ".local/storage-enrollments").glob("*/previous-provisioning.json"))
        self.assertEqual(len(backups), 1)
        self.assertEqual(backups[0].read_bytes(), original)

    def test_wrong_old_inode_device_or_new_volume_acknowledgment_never_writes(self):
        original = self.path.read_bytes()
        changes = ({"previous_device": 0}, {"previous_inode": 0},
                   {"filesystem_uuid": str(uuid.UUID(int=1))}, {"subvolume_uuid": str(uuid.UUID(int=1))})
        with patch.object(storage, "observe", return_value=(self.info, IDENTITY)):
            for change in changes:
                with self.subTest(change=change), self.assertRaises(DevctlError):
                    self.invoke(**change)
                self.assertEqual(self.path.read_bytes(), original)
                self.assertFalse((self.root / ".local/storage-enrollments").exists())

    def test_retained_control_state_denies_before_material_validation(self):
        for path in (self.root / ".local/vm/process.json", self.config.paths["qmp_socket"], self.config.paths["serial_socket"]):
            path.write_bytes(b"retained control fixture")
            with patch.object(vm, "_load_record_material") as load, self.assertRaises(DevctlError) as caught:
                self.invoke()
            self.assertEqual(caught.exception.code, "VM_NOT_STOPPED")
            load.assert_not_called()
            path.unlink()

    def test_changed_media_cannot_be_rebound(self):
        Path(self.record["seed_iso"]).write_bytes(b"changed seed fixture")
        with patch.object(storage, "observe") as observe, self.assertRaises(DevctlError) as caught:
            self.invoke()
        self.assertEqual(caught.exception.code, "BOOTSTRAP_DIGEST_MISMATCH")
        observe.assert_not_called()

    def test_bound_disk_rejects_inode_filesystem_or_subvolume_substitution(self):
        bound = {**self.record, "storage_identity": IDENTITY}
        for field, value in (("filesystem_uuid", str(uuid.UUID(int=1))),
                             ("subvolume_uuid", str(uuid.UUID(int=2))), ("subvolume_id", 256)):
            with patch.object(storage, "observe", return_value=(self.info, {**IDENTITY, field: value})), self.assertRaises(DevctlError):
                storage.verify(self.disk, bound)
        with patch.object(storage, "observe", return_value=(self.info, IDENTITY)), self.assertRaises(DevctlError):
            storage.verify(self.disk, {**bound, "disk_inode": self.info.st_ino + 1})

    def test_unsupported_filesystem_retains_device_check_and_cannot_rebind(self):
        with patch.object(storage, "observe", return_value=(self.info, None)):
            storage.verify(self.disk, {**self.record, "disk_device": self.info.st_dev})
            with self.assertRaises(DevctlError):
                self.invoke()
            with self.assertRaises(DevctlError):
                storage.verify(self.disk, {**self.record, "storage_identity": IDENTITY})

    def test_read_only_ioctl_decoder_uses_filesystem_and_subvolume_uuids(self):
        calls = []
        def ioctl(fd, request, buffer, mutate):
            calls.append(request)
            self.assertTrue(mutate)
            self.assertEqual(os.fstat(fd).st_ino, self.info.st_ino)
            if request == storage.FS_INFO:
                buffer[16:32] = uuid.UUID(IDENTITY["filesystem_uuid"]).bytes
            else:
                struct.pack_into("=Q", buffer, 0, IDENTITY["subvolume_id"])
                buffer[296:312] = uuid.UUID(IDENTITY["subvolume_uuid"]).bytes
        with patch.object(storage.fcntl, "ioctl", side_effect=ioctl):
            info, identity = storage.observe(self.disk)
        self.assertEqual(identity, IDENTITY)
        self.assertEqual(info.st_ino, self.info.st_ino)
        self.assertEqual(calls, [storage.FS_INFO, storage.SUBVOL_INFO])
        self.assertEqual(self.disk.read_bytes(), b"unchanged disk fixture")

    def test_ioctl_permission_failure_does_not_fall_back_to_device_number(self):
        with patch.object(storage.fcntl, "ioctl", side_effect=OSError(errno.EACCES, "denied")), self.assertRaises(OSError):
            storage.observe(self.disk)

    def test_symlink_and_path_substitution_are_rejected(self):
        link = self.disk.with_name("link.qcow2")
        link.symlink_to(self.disk)
        with self.assertRaises(OSError):
            storage.observe(link)
        def replace_disk(*args):
            self.disk.rename(self.disk.with_name("original.qcow2"))
            self.disk.write_bytes(b"replacement fixture")
            raise OSError(errno.ENOTTY, "fixture unsupported filesystem")
        with patch.object(storage.fcntl, "ioctl", side_effect=replace_disk), self.assertRaises(DevctlError):
            storage.observe(self.disk)

    def test_explicit_cli_requires_all_identity_acknowledgments(self):
        with self.assertRaises(DevctlError):
            parser().parse_args(["vm", "rebind-storage"])
        args = parser().parse_args(["vm", "rebind-storage", "--previous-device", "60", "--previous-inode", "123",
                                   "--filesystem-uuid", IDENTITY["filesystem_uuid"], "--subvolume-uuid", IDENTITY["subvolume_uuid"]])
        self.assertEqual(args.previous_device, 60)
        self.assertEqual(args.previous_inode, 123)

    def test_invalid_acknowledgments_do_not_create_receipts(self):
        for change in ({"previous_device": True}, {"previous_inode": -1}, {"filesystem_uuid": "bad"}):
            with self.subTest(change=change), self.assertRaises(DevctlError):
                self.invoke(**change)
        self.assertFalse((self.root / ".local/storage-enrollments").exists())

    def test_malformed_persistent_identity_cannot_disable_device_checks(self):
        for identity in (None, {}, {**IDENTITY, "schema_version": True}, {**IDENTITY, "subvolume_id": True},
                         {**IDENTITY, "filesystem_uuid": "bad"}, {**IDENTITY, "extra": 1}):
            with patch.object(storage, "observe", return_value=(self.info, None)), self.assertRaises(DevctlError):
                storage.verify(self.disk, {**self.record, "storage_identity": identity})

    def test_changed_configuration_cannot_rebind(self):
        changed = VMConfig.from_data(self.root, {**self.config.values, "ssh_port": 2223})
        with patch.object(storage, "observe") as observe, self.assertRaises(DevctlError) as caught:
            storage.rebind(changed, self.record["disk_device"], self.record["disk_inode"],
                           IDENTITY["filesystem_uuid"], IDENTITY["subvolume_uuid"])
        self.assertEqual(caught.exception.code, "TARGET_MISMATCH")
        observe.assert_not_called()

    def test_dangling_control_link_still_denies_enrollment(self):
        (self.root / ".local/vm/process.json").symlink_to("absent-process.json")
        with self.assertRaises(DevctlError) as caught:
            self.invoke()
        self.assertEqual(caught.exception.code, "VM_NOT_STOPPED")

    def test_inode_replacement_before_commit_keeps_original_record(self):
        original = self.path.read_bytes()
        changed = os.stat_result((self.info.st_mode, self.info.st_ino + 1, self.info.st_dev,
                                  self.info.st_nlink, self.info.st_uid, self.info.st_gid,
                                  self.info.st_size, self.info.st_atime, self.info.st_mtime, self.info.st_ctime))
        with patch.object(storage, "observe", side_effect=[(self.info, IDENTITY), (changed, IDENTITY)]), self.assertRaises(DevctlError):
            self.invoke()
        self.assertEqual(self.path.read_bytes(), original)

    def test_external_provider_cannot_use_owned_storage_enrollment(self):
        config = VMConfig.from_data(self.root, {**self.config.values, "provider": "external"})
        with self.assertRaises(DevctlError) as caught:
            storage.rebind(config, self.record["disk_device"], self.record["disk_inode"],
                           IDENTITY["filesystem_uuid"], IDENTITY["subvolume_uuid"])
        self.assertEqual(caught.exception.code, "UNSUPPORTED_CAPABILITY")
