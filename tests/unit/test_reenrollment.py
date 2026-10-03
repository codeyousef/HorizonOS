"""Synthetic host trust fixtures; real SSH replacement is qualified separately."""
import base64
import json
import io
from contextlib import redirect_stdout
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from aios_dev import guest, reenrollment
from aios_dev.cli import main, parser
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError
from aios_dev.provision import write_json_new
from test_guest import EXAMPLE, EXPECTED, IDENTITY, KEY

NEW_UUID = "55555555-5555-4555-8555-555555555555"
NEW_KEY = "ssh-ed25519 " + base64.b64encode(b"\x00\x00\x00\x0bssh-ed25519\x00\x00\x00\x20" + b"b" * 32).decode()


class ReenrollmentTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.config = VMConfig.from_data(self.root, {**EXAMPLE, "provider": "external"})
        (self.root / ".local/ssh").mkdir(parents=True, mode=0o700)
        (self.root / ".local").chmod(0o700)
        self.config.paths["identity_file"].write_text("SYNTHETIC PRIVATE KEY FIXTURE")
        self.config.paths["identity_file"].chmod(0o600)
        guest.install_trust(self.config, *guest.public_key(KEY), EXPECTED, {"kind": "console-fixture"})
        write_json_new(self.root / ".local/enrollment.json", {"schema_version":1,"identity":IDENTITY,
                       "host_key_fingerprint":guest.public_key(KEY)[1]})
        self.expected = {**EXPECTED, "installation_uuid": NEW_UUID}
        self.identity = {**IDENTITY, "installation_uuid": NEW_UUID, "machine_id": "b"*32,
                         "boot_id": "66666666-6666-4666-8666-666666666666"}
        self.console = self.root / ".local/ssh/reinstalled-console.json"
        self.material()
        self.files = reenrollment.destinations(self.config)
        self.before = [p.read_bytes() for p in self.files]

    def material(self, key=NEW_KEY, expected=None):
        self.console.write_text(json.dumps({"schema_version":1,"host_public_key":key,
            "fingerprint":guest.public_key(key)[1], **(expected or self.expected)}))
        self.console.chmod(0o600)

    def call(self, config=None, prior=EXPECTED["installation_uuid"]):
        return reenrollment.run(config or self.config, prior, ".local/ssh/reinstalled-console.json")

    def unchanged(self):
        self.assertEqual([p.read_bytes() for p in self.files], self.before)

    def test_normal_enrollment_never_accepts_replacement_trust(self):
        with patch.object(guest, "identity_response") as ssh, self.assertRaises(DevctlError):
            guest.enroll(self.config, ".local/ssh/reinstalled-console.json")
        ssh.assert_not_called()
        self.unchanged()

    def test_prior_acknowledgement_and_distinct_installation_precede_ssh(self):
        with patch.object(guest, "identity_response") as ssh:
            for prior in ("not-uuid", "77777777-7777-4777-8777-777777777777", NEW_UUID):
                with self.subTest(prior=prior), self.assertRaises(DevctlError):
                    self.call(prior=prior)
            self.material(expected=EXPECTED)
            with self.assertRaises(DevctlError) as caught:
                self.call()
            self.assertEqual(caught.exception.code, "REINSTALL_IDENTITY_UNCHANGED")
        ssh.assert_not_called()
        self.unchanged()

    def test_new_pinned_ssh_target_is_verified_before_replacing_trust(self):
        for field,value in (("os_id","cachyos"),("installation_uuid",EXPECTED["installation_uuid"]),
                            ("dmi_uuid",NEW_UUID),("guest_role","production"),("disk_serial","HOST_DISK")):
            with self.subTest(field=field), patch.object(guest,"identity_response",return_value={**self.identity,field:value}), self.assertRaises(DevctlError):
                self.call()
            self.unchanged()
            self.assertFalse((self.root / reenrollment.PENDING).exists())

    def test_console_fingerprint_tampering_never_contacts_ssh(self):
        data=json.loads(self.console.read_text());data["fingerprint"]="SHA256:wrong";self.console.write_text(json.dumps(data))
        with patch.object(guest,"identity_response") as ssh, self.assertRaises(DevctlError):
            self.call()
        ssh.assert_not_called()
        self.unchanged()

    def test_success_preserves_private_key_and_rejects_old_identity(self):
        key_before=self.config.paths["identity_file"].read_bytes()
        with patch.object(guest,"identity_response",return_value=self.identity) as ssh:
            status,result=self.call()
        self.assertEqual(status,0)
        self.assertTrue(result["prior_deployment_intents_invalidated"])
        self.assertFalse(result["guest_mutation_performed"])
        self.assertEqual(self.config.paths["identity_file"].read_bytes(),key_before)
        self.assertEqual(guest.load_trust(self.config)["expected"],self.expected)
        self.assertNotEqual(ssh.call_args.args[0].paths["known_hosts_file"],self.config.paths["known_hosts_file"])
        self.assertTrue(Path(result["artifact_path"]).is_file())
        self.assertFalse((self.root/reenrollment.PENDING).exists())
        with patch.object(guest,"identity_response",return_value=IDENTITY), self.assertRaises(DevctlError):
            guest.enrolled_identity(self.config)
        with patch.object(guest,"identity_response",return_value=self.identity):
            self.assertEqual(guest.enrolled_identity(self.config)[1],self.identity)

    def test_interrupted_update_blocks_all_normal_operations_then_resumes(self):
        original=reenrollment.replace_file
        def interrupted(path,data,operation):
            if path.name=="trust.json":raise OSError("simulated host interruption")
            original(path,data,operation)
        with patch.object(guest,"identity_response",return_value=self.identity), patch.object(reenrollment,"replace_file",side_effect=interrupted), self.assertRaises(OSError):
            self.call()
        self.assertTrue((self.root/reenrollment.PENDING).is_file())
        with patch.object(guest,"identity_response") as ssh:
            for operation in (lambda:guest.load_trust(self.config),lambda:guest.enroll(self.config),lambda:guest.pin_external(self.config,".local/ssh/reinstalled-console.json")):
                with self.assertRaises(DevctlError) as caught:operation()
                self.assertEqual(caught.exception.code,"REENROLLMENT_INCOMPLETE")
        ssh.assert_not_called()
        with patch.object(guest,"identity_response",return_value=self.identity):
            self.assertEqual(self.call()[0],0)
        self.assertFalse((self.root/reenrollment.PENDING).exists())

    def test_interrupted_request_or_staged_evidence_change_is_denied(self):
        with patch.object(guest,"identity_response",return_value=self.identity), patch.object(reenrollment,"replace_file",side_effect=OSError("interrupted")), self.assertRaises(OSError):
            self.call()
        self.material(key=KEY)
        with patch.object(guest,"identity_response") as ssh, self.assertRaises(DevctlError) as caught:
            self.call()
        self.assertEqual(caught.exception.code,"REENROLLMENT_REQUEST_CHANGED")
        ssh.assert_not_called()
        self.material()
        journal=json.loads((self.root/reenrollment.PENDING).read_text())
        stage=self.root/".local/ssh/re-enrollments"/journal["operation_id"]
        (stage/"known_hosts").write_text("tampered")
        with patch.object(guest,"identity_response") as ssh, self.assertRaises(DevctlError) as caught:
            self.call()
        self.assertEqual(caught.exception.code,"REENROLLMENT_EVIDENCE_CHANGED")
        ssh.assert_not_called()

    def test_new_endpoint_configuration_requires_verified_console_identity(self):
        changed=VMConfig.from_data(self.root,{**self.config.values,"ssh_port":3333})
        with patch.object(guest,"identity_response",return_value=self.identity):
            self.assertEqual(self.call(changed)[0],0)
        self.assertEqual(guest.load_trust(changed)["configuration"]["ssh_port"],3333)
        with self.assertRaises(DevctlError):guest.load_trust(self.config)

    def test_missing_console_ack_and_pin_only_cli_are_distinct(self):
        parsed=parser().parse_args(["enroll","--re-enroll",EXPECTED["installation_uuid"],"--trust-file",".local/ssh/console.json"])
        self.assertEqual(parsed.re_enroll,EXPECTED["installation_uuid"])
        with patch.object(guest,"identity_response") as ssh, self.assertRaises(DevctlError):
            reenrollment.run(self.config,EXPECTED["installation_uuid"])
        ssh.assert_not_called()
        output=io.StringIO()
        with patch.object(guest,"identity_response") as ssh, redirect_stdout(output):
            status=main(["enroll","--re-enroll",EXPECTED["installation_uuid"],"--pin-console-only","--json"])
        self.assertEqual(status,2)
        self.assertEqual(json.loads(output.getvalue())["error"]["code"],"INVALID_ARGUMENT")
        ssh.assert_not_called()

    def test_managed_provider_requires_verified_console_material(self):
        managed=VMConfig.from_data(self.root,{**self.config.values,"provider":"qemu"})
        material=(*guest.public_key(NEW_KEY),self.expected,{"kind":"verified-qemu-console","serial_sha256":"c"*64})
        with patch.object(guest,"console_material",return_value=material) as console, patch.object(guest,"identity_response",return_value=self.identity):
            self.assertEqual(reenrollment.run(managed,EXPECTED["installation_uuid"])[0],0)
        console.assert_called_once_with(managed)
        self.assertEqual(guest.load_trust(managed)["source"]["kind"],"verified-qemu-console")

    def test_pending_active_file_drift_stops_before_any_replacement(self):
        with patch.object(guest,"identity_response",return_value=self.identity), patch.object(reenrollment,"replace_file",side_effect=OSError("interrupted")), self.assertRaises(OSError):
            self.call()
        self.files[2].write_bytes(b"changed active enrollment")
        known_before=self.files[0].read_bytes()
        with patch.object(guest,"identity_response",return_value=self.identity), self.assertRaises(DevctlError) as caught:
            self.call()
        self.assertEqual(caught.exception.code,"REENROLLMENT_STATE_CHANGED")
        self.assertEqual(self.files[0].read_bytes(),known_before)
        self.assertTrue((self.root/reenrollment.PENDING).is_file())


if __name__=="__main__":unittest.main()
