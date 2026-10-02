"""Host trust/identity fixtures; these are not real guest acceptance."""
import base64
import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError
from aios_dev.guest import enroll, identity_response, install_trust, load_trust, pin_external, public_key, ssh_arguments, verify_identity

EXAMPLE = json.loads((ROOT / "dev/vm.example.json").read_text())
EXPECTED = {"guest_uuid": "11111111-1111-4111-8111-111111111111", "installation_uuid": "22222222-2222-4222-8222-222222222222",
            "guest_role": "development", "disk_serial": "AIOS_DEV_ROOT", "management_channel": "ssh-development"}
IDENTITY = {"schema_version": 1, "os_id": "nixos", "os_version": "26.05", "hostname": "aios-dev", "dmi_uuid": EXPECTED["guest_uuid"],
            "installation_uuid": EXPECTED["installation_uuid"], "guest_role": "development", "boot_id": "33333333-3333-4333-8333-333333333333",
            "machine_id": "a" * 32, "current_system": "/nix/store/" + "a" * 32 + "-nixos-system-aios-dev-26.05",
            "disk_serial": "AIOS_DEV_ROOT", "management_channel": "ssh-development"}
KEY = "ssh-ed25519 " + base64.b64encode(b"\x00\x00\x00\x0bssh-ed25519\x00\x00\x00\x20" + b"a" * 32).decode()


class GuestTrustTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="aios-guest-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.config = VMConfig.from_data(self.root, EXAMPLE)
        (self.root / ".local/ssh").mkdir(parents=True, mode=0o700)
        (self.root / ".local").chmod(0o700)
        self.config.paths["identity_file"].write_text("PRIVATE KEY FIXTURE ONLY")
        self.config.paths["identity_file"].chmod(0o600)

    def pin(self):
        key, fingerprint = public_key(KEY)
        return install_trust(self.config, key, fingerprint, EXPECTED, {"kind": "console-fixture"})

    def test_missing_console_trust_cannot_contact_guest(self):
        with patch("aios_dev.guest.pin_console", side_effect=DevctlError(3, "MISSING", "fixture")), patch("aios_dev.guest.identity_response") as ssh:
            with self.assertRaises(DevctlError):
                enroll(self.config)
        ssh.assert_not_called()

    def test_forged_identity_is_denied_before_enrollment_side_effects(self):
        self.pin()
        for field, invalid in (("os_id", "cachyos"), ("dmi_uuid", EXPECTED["installation_uuid"]), ("installation_uuid", EXPECTED["guest_uuid"]),
                               ("guest_role", "production"), ("disk_serial", "HOST_DISK"), ("management_channel", "local-host"), ("current_system", "/etc/nixos")):
            forged = {**IDENTITY, field: invalid}
            with self.subTest(field=field), patch("aios_dev.guest.identity_response", return_value=forged), self.assertRaises(DevctlError):
                enroll(self.config)
            self.assertFalse((self.root / ".local/enrollment.json").exists())

    def test_pinned_key_drift_stops_before_ssh(self):
        self.pin()
        self.config.paths["known_hosts_file"].write_text("changed trust fixture")
        with patch("aios_dev.guest.identity_response") as ssh, self.assertRaises(DevctlError):
            enroll(self.config)
        ssh.assert_not_called()

    def test_config_drift_stops_before_ssh(self):
        self.pin()
        changed = VMConfig.from_data(self.root, {**EXAMPLE, "ssh_port": 2223})
        with self.assertRaises(DevctlError):
            load_trust(changed)

    def test_duplicate_or_boolean_schema_and_extra_fields_are_denied(self):
        for value in ({**IDENTITY, "schema_version": True}, {**IDENTITY, "admin": True}, {**IDENTITY, "boot_id": "not-uuid"}):
            with self.assertRaises(DevctlError):
                verify_identity(value, EXPECTED)

    def test_changed_machine_denied_but_new_boot_is_allowed(self):
        verify_identity({**IDENTITY, "boot_id": "44444444-4444-4444-8444-444444444444"}, EXPECTED, IDENTITY, mutation=True)
        with self.assertRaises(DevctlError):
            verify_identity({**IDENTITY, "machine_id": "b" * 32}, EXPECTED, IDENTITY)

    def test_ssh_has_no_password_root_or_forwarding_fallback(self):
        args = ssh_arguments(self.config)
        for required in ("StrictHostKeyChecking=yes", "IdentityAgent=none", "ClearAllForwardings=yes", "PasswordAuthentication=no", "GlobalKnownHostsFile=/dev/null"):
            self.assertIn(required, args)
        self.assertEqual(args[-1], "/run/current-system/sw/bin/aios-guest-identity")
        self.assertIn("dev@127.0.0.1", args)

    def test_changed_host_key_requires_explicit_reenrollment(self):
        self.pin()
        other = "ssh-ed25519 " + base64.b64encode(b"\x00\x00\x00\x0bssh-ed25519\x00\x00\x00\x20" + b"b" * 32).decode()
        with self.assertRaises(DevctlError):
            install_trust(self.config, *public_key(other), EXPECTED, {"kind": "console-fixture"})

    def test_oversized_guest_stream_is_killed_before_parsing(self):
        with patch("aios_dev.guest.ssh_arguments", return_value=[sys.executable, "-c", "import sys; sys.stdout.write('x'*65537)"]):
            with self.assertRaises(DevctlError) as caught:
                identity_response(self.config)
        self.assertEqual(caught.exception.code, "IDENTITY_RESPONSE_LIMIT")

    def test_duplicate_guest_identity_fields_are_not_accepted(self):
        with patch("aios_dev.guest.ssh_arguments", return_value=[sys.executable, "-c", 'print(\'{"schema_version":0,"schema_version":1}\')']):
            with self.assertRaises(DevctlError) as caught:
                identity_response(self.config)
        self.assertEqual(caught.exception.code, "INVALID_GUEST_IDENTITY")

    def test_external_adoption_requires_matching_explicit_console_material(self):
        config = VMConfig.from_data(self.root, {**EXAMPLE, "provider": "external", "ssh_host": "guest.example.org"})
        path = self.root / ".local/ssh/console.json"
        key, fingerprint = public_key(KEY)
        path.write_text(json.dumps({"schema_version": 1, "host_public_key": key, "fingerprint": fingerprint, **EXPECTED}))
        path.chmod(0o600)
        trust = pin_external(config, ".local/ssh/console.json")
        self.assertEqual(trust["expected"], EXPECTED)
        with patch("aios_dev.guest.identity_response", return_value=IDENTITY):
            status, result = enroll(config)
        self.assertEqual(status, 0)
        self.assertTrue(result["identity_verified"])

    def test_external_wrong_fingerprint_cannot_write_known_hosts(self):
        config = VMConfig.from_data(self.root, {**EXAMPLE, "provider": "external", "ssh_host": "guest.example.org"})
        path = self.root / ".local/ssh/console.json"
        path.write_text(json.dumps({"schema_version": 1, "host_public_key": KEY, "fingerprint": "SHA256:wrong", **EXPECTED}))
        path.chmod(0o600)
        with self.assertRaises(DevctlError):
            pin_external(config, ".local/ssh/console.json")
        self.assertFalse(config.paths["known_hosts_file"].exists())


if __name__ == "__main__":
    unittest.main()
