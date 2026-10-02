"""Host runner boundary fixtures, never real installer/OS acceptance."""
import base64
import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from aios_dev import acceptance, provision, vm
from aios_dev.cli import main
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError, ExitCode

EXAMPLE = json.loads((ROOT / "dev/vm.example.json").read_text())
RUN = "abcdef12-1111-4111-8111-111111111111"
GUEST = "22222222-2222-4222-8222-222222222222"
INSTALL = "33333333-3333-4333-8333-333333333333"


class AcceptanceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="a-")
        self.addCleanup(self.temp.cleanup)
        self.owner = Path(self.temp.name)
        # Fixtures replace only the storage root; production has no override.
        self.storage = patch.object(acceptance, "STORAGE_ROOT", self.owner)
        self.storage.start()
        self.addCleanup(self.storage.stop)
        self.root = provision.private_directory(self.owner, ".local/a/abcdef12/d")
        self.config = VMConfig.from_data(self.root, EXAMPLE)
        provision.private_directory(self.root, ".local")
        self.record = {"plan": {"guest_uuid": GUEST, "installation_uuid": INSTALL}, "disk_device": 1, "disk_inode": 2}
        self.binding = {"schema_version": 1, "owner_workspace": str(self.owner), "run_id": RUN, "case": "wrong-disk", "configuration": self.config.values,
                        "guest_uuid": GUEST, "installation_uuid": INSTALL, "disk_device": 1, "disk_inode": 2,
                        "source_head": "a" * 40, "source_dirty": False, "source_snapshot_digest": "b" * 64}
        provision.write_json_new(self.root / ".local/disposable.json", self.binding)

    def test_binding_rejects_main_workspace_and_identity_drift(self):
        self.assertEqual(acceptance.validate_disposable(self.config, self.record, "wrong-disk"), self.binding)
        main_config = VMConfig.from_data(self.owner, EXAMPLE)
        provision.write_json_new(self.owner / ".local/disposable.json", self.binding)
        with self.assertRaises(DevctlError):
            acceptance.validate_disposable(main_config, self.record, "wrong-disk")
        for changed in ({**self.record, "disk_inode": 3}, {**self.record, "plan": {**self.record["plan"], "guest_uuid": INSTALL}}):
            with self.subTest(changed=changed), self.assertRaises(DevctlError) as caught:
                acceptance.validate_disposable(self.config, changed, "wrong-disk")
            self.assertEqual(caught.exception.exit_code, ExitCode.TARGET_MISMATCH)
        with self.assertRaises(DevctlError):
            acceptance.validate_disposable(self.config, self.record, "reinstall")

    def test_binding_cannot_be_symlink_or_public(self):
        marker = self.root / ".local/disposable.json"
        marker.chmod(0o644)
        with self.assertRaises(DevctlError):
            acceptance.validate_disposable(self.config, self.record, "wrong-disk")
        marker.unlink()
        marker.symlink_to(self.owner / ".local/missing")
        with self.assertRaises(DevctlError):
            acceptance.validate_disposable(self.config, self.record, "wrong-disk")

    def test_unregistered_or_non_disposable_case_sends_no_console_input(self):
        client = object.__new__(vm.QMP)
        client.process = {"arguments": ["if=none,id=installer,media=cdrom,readonly=on,file=/iso"]}
        record = {**self.record, "media": {"path": "/iso"}, "seed_iso": "/seed"}
        with patch.object(vm, "verify_block"), patch.object(vm, "load_record", return_value=record), patch.object(client, "_console_sequence") as send:
            for case in ("shell", "reinstall"):
                with self.assertRaises(DevctlError):
                    client.bootstrap_console(self.config, qualification=case)
            with self.assertRaises(DevctlError) as caught:
                client.bootstrap_console(self.config, qualification="wrong-disk")
            self.assertEqual(caught.exception.code, "QUALIFICATION_LAUNCH_MISMATCH")
        send.assert_not_called()

    def test_wrong_serial_override_is_registered_and_installer_only(self):
        config = VMConfig.from_data(self.owner, EXAMPLE)
        record = {**self.record, "seed_iso": "/seed", "firmware_code": "/firmware", "media": {"path": "/iso"}}
        with patch.object(acceptance, "validate_disposable"), patch.object(vm.shutil, "which", return_value="/qemu"):
            args = vm.qemu_arguments(config, record, "none", qualification="wrong-disk")
            self.assertIn("virtio-blk-pci,drive=rootdisk,serial=AIOS_WRONG_ROOT,bootindex=2", args)
            ordinary = vm.qemu_arguments(config, record, "none")
            self.assertIn("virtio-blk-pci,drive=rootdisk,serial=AIOS_DEV_ROOT,bootindex=2", ordinary)
            with self.assertRaises(DevctlError):
                vm.qemu_arguments(config, record, "none", bootstrap=False, qualification="wrong-disk")

    def test_registered_scripts_have_valid_bash_and_measure_whole_disk(self):
        with patch.object(acceptance, "validate_disposable"):
            for case in acceptance.CASES:
                command, payload = acceptance.console_command(self.config, self.record, case)
                script = base64.b64decode(payload)
                self.assertEqual(subprocess.run(["bash", "-n"], input=script, capture_output=True, timeout=5).returncode, 0)
                self.assertEqual(script.count(b"sha256sum /dev/vda"), 2)
                self.assertIn(b'test "$status" = 4', script)
                self.assertIn(b'test "$before" = "$after"', script)
                self.assertIn(GUEST.encode(), script)
                self.assertTrue(vm.console_keys(command))
        with self.assertRaises(DevctlError):
            acceptance.console_command(self.config, self.record, "shell")

    def test_explicit_scope_cannot_claim_full_integration_acceptance(self):
        for args, expected in ((["test", "--suite", "integration"], 9),
                               (["test", "--suite", "unit", "--bootstrap-case", "all"], 2),
                               (["test", "--suite", "integration", "--bootstrap-case", "all", "--detach"], 2),
                               (["test", "--suite", "integration", "--bootstrap-case", "shell"], 2)):
            with contextlib.redirect_stdout(io.StringIO()), patch.object(acceptance, "run_bootstrap_guards") as run:
                self.assertEqual(main([*args, "--json"]), expected)
            run.assert_not_called()

    def test_verified_owner_required_before_fixture_creation(self):
        owner = VMConfig.from_data(self.owner, EXAMPLE)
        with patch("aios_dev.guest.enrolled_identity", side_effect=DevctlError(4, "MISMATCH", "fixture")), patch.object(acceptance, "prepare_case") as prepare:
            with self.assertRaises(DevctlError):
                acceptance.run_bootstrap_guards(owner)
        prepare.assert_not_called()


if __name__ == "__main__":
    unittest.main()
