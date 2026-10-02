"""Offline account/storage parser boundaries; these are host fixtures."""
import base64
import importlib.util
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
from aios_dev import vm, provision
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError, ExitCode

spec = importlib.util.spec_from_file_location("layout_fixture", ROOT / "tools/guest/layout_audit.py")
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)
INSTALL = "33333333-3333-4333-8333-333333333333"
GUEST = "22222222-2222-4222-8222-222222222222"
EXAMPLE = json.loads((ROOT / "dev/vm.example.json").read_text())


class LayoutTests(unittest.TestCase):
    def accounts(self):
        passwd = "root:x:0:0:root:/root:/bin/bash\ndev:x:1000:100:dev:/home/dev:/bin/bash\ntester:x:1001:100:tester:/home/tester:/bin/bash\n"
        groups = "root:x:0:\nwheel:x:1:tester\nusers:x:100:\n"
        shadow = "root:!:1:0:99999:7:::\ndev:!:1:0:99999:7:::\ntester:$y$fixture-hash:1:0:99999:7:::\n"
        for uid, name in enumerate(audit.SERVICES, 10):
            passwd += f"{name}:x:{uid}:{uid}::/var/empty:/nix/store/fixture/bin/nologin\n"
            groups += f"{name}:x:{uid}:\n"
            shadow += f"{name}:!:1:0:99999:7:::\n"
        return passwd, groups, shadow

    def test_account_evidence_excludes_actual_password_hash(self):
        result = audit.account_report(*self.accounts())
        self.assertEqual(result["tester"]["password_state"], "hashed")
        self.assertNotIn("fixture-hash", json.dumps(result))
        self.assertNotIn("wheel", result["dev"]["groups"])

    def test_wheel_shell_and_plaintext_credentials_are_denied(self):
        passwd, groups, shadow = self.accounts()
        for values in ((passwd, groups.replace("wheel:x:1:tester", "wheel:x:1:tester,dev"), shadow),
                       (passwd.replace("fixture/bin/nologin", "fixture/bin/bash"), groups, shadow),
                       (passwd, groups, shadow.replace("$y$fixture-hash", "plaintext-fixture"))):
            with self.assertRaises(audit.Denied) as caught:
                audit.account_report(*values)
            self.assertNotIn("plaintext-fixture", str(caught.exception))

    def fstab(self):
        value = "# public fixture\n"
        for path, name in (("/", "root"), ("/home", "home"), ("/nix", "nix"), ("/var", "var")):
            value += f"/dev/disk/by-uuid/{INSTALL} {path} btrfs subvol=@{name} 0 0\n"
        return value + "/dev/disk/by-uuid/efi /boot vfat fmask=0077,dmask=0077 0 2\n"

    def test_wrong_uuid_missing_subvolume_swap_and_public_efi_are_denied(self):
        self.assertEqual(len(audit.mount_report(self.fstab(), INSTALL)), 5)
        for value in (self.fstab().replace(INSTALL, GUEST), self.fstab().replace("subvol=@home", "defaults"),
                      self.fstab() + "/dev/vda3 none swap defaults 0 0\n", self.fstab().replace("fmask=0077", "fmask=0022")):
            with self.assertRaises(audit.Denied):
                audit.mount_report(value, INSTALL)

    def test_nix_trusted_users_and_sandbox_are_enforced(self):
        good = "trusted-users = root\nsandbox = true\n"
        self.assertTrue(audit.nix_report(good)["sandbox"])
        for value in (good.replace("root", "root dev"), good.replace("true", "false"), good + "trusted-users = root\n"):
            with self.assertRaises(audit.Denied):
                audit.nix_report(value)

    def test_symlink_and_public_secret_metadata_rejected_without_reading_secret(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "secret"
            path.write_text("fixture-content-not-an-actual-credential")
            path.chmod(0o644)
            values = list(path.lstat())
            values[4] = 0
            with self.assertRaises(audit.Denied) as caught, patch.object(Path, "lstat", return_value=os.stat_result(values)), patch.object(Path, "read_text", side_effect=AssertionError("credential read forbidden")):
                audit.credential_metadata(path, secret=True)
            self.assertEqual(str(caught.exception), "credential-permissions")
            link = Path(temp) / "link"
            link.symlink_to(path)
            with self.assertRaises(audit.Denied):
                audit.credential_metadata(link, secret=True)

    def test_audit_wrapper_is_valid_bash_and_binds_guest_pin_before_chroot(self):
        with patch("aios_dev.guest.load_trust", return_value={"host_key_fingerprint": "SHA256:" + "a" * 43}):
            _, payload = vm.audit_console_command(None, {"plan": {"guest_uuid": GUEST, "installation_uuid": INSTALL}})
        script = base64.b64decode(payload)
        self.assertEqual(subprocess.run(["bash", "-n"], input=script, capture_output=True, timeout=5).returncode, 0)
        self.assertIn(b"ro,nologreplay", script)
        self.assertLess(script.index(b"ssh-keygen -lf"), script.index(b"chroot /mnt"))

    def test_graceful_timeout_never_forces_vm_or_removes_control_state(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            config = VMConfig.from_data(root, EXAMPLE)
            provision.private_directory(root, ".local/vm")
            state = root / ".local/vm/process.json"
            provision.write_json_new(state, {"pid": 42})
            client = Mock()
            with patch.object(vm, "load_record", return_value={"plan": {"guest_uuid": GUEST}}), patch.object(vm, "QMP", return_value=client), patch.object(vm, "verify_block"), patch.object(vm, "verify_process"), patch.object(vm.time, "sleep"):
                with self.assertRaises(DevctlError) as caught:
                    vm.stop(config, graceful=True)
            self.assertEqual(caught.exception.exit_code, ExitCode.TIMEOUT)
            client.command.assert_called_once_with("system_powerdown")
            self.assertTrue(state.exists())


if __name__ == "__main__":
    unittest.main()
