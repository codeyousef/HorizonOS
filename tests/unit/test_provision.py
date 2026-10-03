"""Host provider fixtures. These do not execute or certify the guest installer."""
import copy
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError, ExitCode
from aios_dev.provision import CHECKSUM_URL, create, digest_file, fetch_media, official_url, operation_lock, prepare_plan, private_directory, run, seed_manifest, source_files, validate_plan
from aios_dev.vm import BOOTSTRAP_CONSOLE, BOOTSTRAP_INSPECT, QMP, console_keys, qemu_arguments, verify_block, verify_process

EXAMPLE = json.loads((ROOT / "dev/vm.example.json").read_text())


class ProvisionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="aios-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.config = VMConfig.from_data(self.root, EXAMPLE)

    def test_plan_persists_uuid_without_creating_disk_or_key(self):
        with patch("aios_dev.provision.run") as execute, patch("aios_dev.provision.fetch_media") as fetch:
            code, data = create(self.config, None)
            again = prepare_plan(self.config)
        self.assertEqual(code, ExitCode.AUTHORIZATION_NEEDED)
        self.assertEqual(data["plan"]["guest_uuid"], again["guest_uuid"])
        self.assertFalse(self.config.paths["disk_image"].exists())
        self.assertFalse(self.config.paths["identity_file"].exists())
        self.assertEqual((self.root / ".local").stat().st_mode & 0o777, 0o700)
        execute.assert_not_called()
        fetch.assert_not_called()

    def test_wrong_uuid_cannot_download_or_mutate_disk(self):
        plan = prepare_plan(self.config)
        with patch("aios_dev.provision.run") as execute, patch("aios_dev.provision.fetch_media") as fetch:
            with self.assertRaises(DevctlError) as caught:
                create(self.config, "00000000-0000-0000-0000-000000000000")
        self.assertEqual(caught.exception.exit_code, ExitCode.TARGET_MISMATCH)
        execute.assert_not_called()
        fetch.assert_not_called()
        self.assertFalse(self.config.paths["disk_image"].exists())

    def test_concurrent_vm_operation_is_denied_before_work(self):
        with operation_lock(self.root), patch("aios_dev.provision.run") as execute:
            with self.assertRaises(DevctlError) as caught:
                create(self.config, None)
        self.assertEqual(caught.exception.code, "VM_OPERATION_BUSY")
        execute.assert_not_called()

    def test_tool_failure_preserves_upstream_exit_without_exposing_output(self):
        with patch("aios_dev.provision.subprocess.run", return_value=subprocess.CompletedProcess([], 42, "sensitive fixture", "sensitive fixture")), self.assertRaises(DevctlError) as caught:
            run(["qemu-img", "info", "fixture"])
        self.assertEqual(caught.exception.details["upstream_exit"], 42)
        self.assertNotIn("sensitive", str(caught.exception) + json.dumps(caught.exception.details))

    def test_existing_disk_is_never_overwritten(self):
        private_directory(self.root, ".local/vm")
        disk = self.config.paths["disk_image"]
        disk.write_bytes(b"existing installed disk fixture")
        with patch("aios_dev.provision.run") as execute:
            with self.assertRaises(DevctlError) as caught:
                create(self.config, None)
        self.assertEqual(caught.exception.code, "REINSTALL_DENIED")
        self.assertEqual(disk.read_bytes(), b"existing installed disk fixture")
        execute.assert_not_called()

    def test_plan_rejects_configuration_drift_and_malformed_uuid(self):
        plan = prepare_plan(self.config)
        changed = VMConfig.from_data(self.root, {**EXAMPLE, "ssh_port": 2223})
        with self.assertRaises(DevctlError):
            validate_plan(changed, plan)
        with self.assertRaises(DevctlError):
            validate_plan(self.config, {**plan, "guest_uuid": "bad"})

    def test_private_directory_does_not_chmod_or_follow_escape(self):
        with tempfile.TemporaryDirectory() as outside:
            path = Path(outside)
            path.chmod(0o755)
            (self.root / ".local").symlink_to(path, target_is_directory=True)
            with self.assertRaises(DevctlError):
                prepare_plan(self.config)
            self.assertEqual(path.stat().st_mode & 0o777, 0o755)

    def test_external_provider_is_unsupported(self):
        config = VMConfig.from_data(self.root, {**EXAMPLE, "provider": "external", "ssh_host": "vm.example.org"})
        with self.assertRaises(DevctlError) as caught:
            create(config, None)
        self.assertEqual(caught.exception.exit_code, ExitCode.UNSUPPORTED_CAPABILITY)

    def test_only_official_https_media_urls(self):
        self.assertEqual(official_url(CHECKSUM_URL), CHECKSUM_URL)
        for url in ("http://releases.nixos.org/x", "https://evil.example/x", "https://releases.nixos.org.evil.example/x", "https://user:secret@releases.nixos.org/x", "https://releases.nixos.org:8443/x"):
            with self.subTest(url=url), self.assertRaises(DevctlError):
                official_url(url)

    def test_cached_media_digest_mismatch_is_denied(self):
        url = "https://releases.nixos.org/nixos/26.05/release/nixos-minimal-26.05.1234.abcdef-x86_64-linux.iso"
        path = self.root / url.rsplit("/", 1)[1]
        path.write_bytes(b"tampered")
        response = io.BytesIO(("0" * 64 + "  " + path.name + "\n").encode())
        response.geturl = lambda: url + ".sha256"
        with patch("aios_dev.provision.urllib.request.build_opener") as builder:
            builder.return_value.open.return_value = response
            with self.assertRaises(DevctlError) as caught:
                fetch_media(self.root)
        self.assertEqual(caught.exception.code, "MEDIA_DIGEST_MISMATCH")
        self.assertEqual(path.read_bytes(), b"tampered")

    def test_bad_download_never_becomes_trusted_iso(self):
        url = "https://releases.nixos.org/nixos/26.05/release/nixos-minimal-26.05.1234.abcdef-x86_64-linux.iso"
        checksum = io.BytesIO(("0" * 64).encode())
        checksum.geturl = lambda: url + ".sha256"
        iso = io.BytesIO(b"wrong fixture")
        iso.geturl = lambda: url
        with patch("aios_dev.provision.urllib.request.build_opener") as builder:
            builder.return_value.open.side_effect = [checksum, iso]
            with self.assertRaises(DevctlError) as caught:
                fetch_media(self.root)
        self.assertEqual(caught.exception.code, "MEDIA_DIGEST_MISMATCH")
        self.assertFalse((self.root / url.rsplit("/", 1)[1]).exists())
        self.assertFalse(list(self.root.glob("*.part")))

    def test_seed_source_rejects_credentials_and_symlinks(self):
        for name in (".local/ssh/private", "tools/secret.pem", "docs/../secret", "/etc/shadow"):
            completed = subprocess.CompletedProcess([], 0, name + "\0", "")
            with patch("aios_dev.provision.run", return_value=completed), self.assertRaises(DevctlError):
                source_files(self.root)
        (self.root / "tools").mkdir()
        (self.root / "tools/link.py").symlink_to("/etc/passwd")
        with patch("aios_dev.provision.run", return_value=subprocess.CompletedProcess([], 0, "tools/link.py\0", "")), self.assertRaises(DevctlError):
            source_files(self.root)

    def test_seed_cannot_supply_existing_machine_enrollment(self):
        name = "nix/machines/aios-dev/enrollment.json"
        target = self.root / name
        target.parent.mkdir(parents=True)
        target.write_text('{"schema_version":1}')
        completed = subprocess.CompletedProcess([], 0, name + "\0", "")
        with patch("aios_dev.provision.run", return_value=completed), self.assertRaisesRegex(DevctlError, "generated only from the verified fresh target"):
            source_files(self.root)

    def test_manifest_digest_covers_public_files(self):
        (self.root / "guest.uuid").write_text("public UUID fixture\n")
        seed_manifest(self.root)
        self.assertEqual((self.root / "manifest.sha256").read_text(), f"{digest_file(self.root / 'guest.uuid')}  guest.uuid\n")

    def record(self):
        return {"plan": prepare_plan(self.config), "seed_iso": str(self.root / ".local/vm/seed.iso"), "firmware_code": "/usr/share/OVMF/OVMF_CODE.fd", "media": {"path": str(self.root / ".local/vm/installer.iso")}}

    def test_qemu_has_bound_identity_and_no_host_mount_or_remote_display(self):
        with patch("aios_dev.vm.shutil.which", return_value="/usr/bin/qemu-system-x86_64"):
            args = qemu_arguments(self.config, self.record(), "gtk")
        self.assertIn("q35,accel=kvm", args)
        self.assertIn("host", args)
        self.assertIn("virtio-vga", args)
        self.assertTrue(any("serial=AIOS_DEV_ROOT" in arg for arg in args))
        self.assertTrue(any("hostfwd=tcp:127.0.0.1:2222-:22" in arg for arg in args))
        self.assertTrue(any("readonly=on,file=/usr/share/OVMF" in arg for arg in args))
        for forbidden in ("-virtfs", "-fsdev", "-spice", "-vnc", "vfio", "0.0.0.0", "-enable-kvm-fallback"):
            self.assertFalse(any(forbidden in arg for arg in args))

    def test_qemu_path_option_injection_rejected(self):
        data = {**EXAMPLE, "disk_image": ".local/vm/root,readonly=on.qcow2"}
        config = VMConfig.from_data(self.root, data)
        with self.assertRaises(DevctlError):
            qemu_arguments(config, self.record(), "gtk")

    def test_pid_reuse_or_argument_drift_denies_control(self):
        expected = {"pid": 42, "uid": os.getuid(), "start_ticks": 123, "arguments": ["qemu"], "executable": "/qemu"}
        for changed in ({**expected, "start_ticks": 124}, {**expected, "arguments": ["other-qemu"]}, {**expected, "uid": os.getuid() + 1}):
            with patch("aios_dev.vm.process_identity", return_value=changed), self.assertRaises(DevctlError) as caught:
                verify_process(expected)
            self.assertEqual(caught.exception.exit_code, ExitCode.TARGET_MISMATCH)

    def test_qmp_wrong_disk_denies_lifecycle_action(self):
        from unittest.mock import Mock
        client = Mock()
        client.command.return_value = [{"device": "rootdisk", "inserted": {"file": "/other/root.qcow2"}}]
        with self.assertRaises(DevctlError) as caught:
            verify_block(client, self.config)
        self.assertEqual(caught.exception.code, "VM_DISK_MISMATCH")

    def test_bootstrap_console_has_no_caller_shell_or_keyboard_rpc(self):
        for registered in (BOOTSTRAP_CONSOLE, BOOTSTRAP_INSPECT):
            self.assertTrue(console_keys(registered))
        client = object.__new__(QMP)
        for forbidden in ("send-key", "human-monitor-command", "blockdev-add", "system_reset"):
            with self.subTest(forbidden=forbidden), self.assertRaises(DevctlError):
                client.command(forbidden)

    def test_keyboard_rejects_control_sequences_before_delivery(self):
        for value in ("echo a\nrm", "\x1b", "\x00", "unicode \u2603"):
            with self.subTest(value=value), self.assertRaises(DevctlError):
                console_keys(value)

    def test_bootstrap_wrong_disk_cannot_send_console_input(self):
        client = object.__new__(QMP)
        with patch("aios_dev.vm.verify_block", side_effect=DevctlError(ExitCode.TARGET_MISMATCH, "VM_DISK_MISMATCH", "fixture")), patch.object(client, "_console_sequence") as send:
            with self.assertRaises(DevctlError):
                client.bootstrap_console(self.config)
        send.assert_not_called()
        self.assertFalse(list(self.root.glob(".local/vm/bootstrap-console*")))

    def test_seed_refresh_cannot_change_running_vm_media(self):
        from aios_dev.provision import refresh_seed
        private_directory(self.root, ".local/vm")
        (self.root / ".local/vm/process.json").write_text("running fixture")
        with patch("aios_dev.vm.load_record", return_value={}), patch("aios_dev.provision.source_files") as collect:
            with self.assertRaises(DevctlError) as caught:
                refresh_seed(self.config)
        self.assertEqual(caught.exception.code, "VM_MUST_BE_STOPPED")
        collect.assert_not_called()


if __name__ == "__main__":
    unittest.main()
