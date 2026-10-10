"""Host transaction fixtures; installed-root qualification is separate."""
import hashlib
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from aios_dev import deploy, sync
from aios_dev.cli import parser
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError
from test_guest import EXAMPLE, IDENTITY

TRANSACTION = "99999999-9999-4999-8999-999999999999"


class HostDeploymentTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.config = VMConfig.from_data(self.root, EXAMPLE)
        self.helper = b"# public helper fixture\n"
        (self.root / "tools/guest").mkdir(parents=True)
        (self.root / "tools/guest/dev_deploy.py").write_bytes(self.helper)
        self.manifest = {"schema_version": 1, "git_head": "a" * 40, "dirty": True, "files": [
            {"path": "tools/guest/dev_deploy.py", "mode": 0o644, "size": len(self.helper), "sha256": hashlib.sha256(self.helper).hexdigest()}]}
        self.digest = sync.contract.validate_manifest(self.manifest)
        (self.root / ".local/snapshots").mkdir(parents=True, mode=0o700)
        (self.root / ".local").chmod(0o700)
        self.record = self.root / ".local/snapshots" / (self.digest + ".json")
        self.record.write_bytes(sync.contract.canonical({"manifest": self.manifest}))
        self.record.chmod(0o600)
        self.source = {"snapshot_digest": self.digest, "identity": IDENTITY}
        self.trust = {"host_key_fingerprint": "SHA256:fixture"}
        self.receipt = {"schema_version": 1, "transaction_id": TRANSACTION, "state": "REGISTERED",
                        "snapshot_digest": self.digest, "identity": IDENTITY, "developer_uid": 1000,
                        "authority": deploy.AUTHORITY, "source_head": self.manifest["git_head"], "source_dirty": True,
                        "file_count": 1, "activation_performed": False, "helper_sha256": "e" * 64,
                        "limitations": ["No activation has occurred; test and commit remain required."],
                        "candidate_closure": None, "candidate_digest": None, "build_source_digest": None,
                        "test_guard_id": None, "commit_guard_id": None, "baseline": None, "committed_identity": None}
        headroom = patch.object(deploy.resources, "require_build_headroom")
        self.headroom = headroom.start()
        self.addCleanup(headroom.stop)
        for module, name, value in ((deploy.guest, "enrolled_identity", (self.trust, IDENTITY)),
                                    (deploy.guest, "ssh_arguments", ["ssh", "pinned", "identity-command"]),
                                    (deploy.sync, "synchronize", (0, self.source)),
                                    (deploy.sync, "exchange", (0, sync.contract.canonical(self.receipt), b""))):
            patcher = patch.object(module, name, return_value=value)
            setattr(self, name, patcher.start())
            self.addCleanup(patcher.stop)

    def call(self, mode="register", acknowledge=True):
        return deploy.run(self.config, mode, TRANSACTION, acknowledge)

    def test_explicit_authority_and_canonical_transaction_precede_transport(self):
        for identifier in ("../escape", "$(false)", True):
            with self.assertRaises(DevctlError):
                deploy.run(self.config, "register", identifier, True)
        with self.assertRaises(DevctlError) as caught:
            self.call(acknowledge=False)
        self.assertEqual(caught.exception.exit_code, 5)
        self.enrolled_identity.assert_not_called()
        self.exchange.assert_not_called()

    def test_intent_survives_disconnect_and_status_uses_fixed_command(self):
        self.exchange.side_effect = TimeoutError
        with self.assertRaises(DevctlError) as caught:
            self.call()
        self.assertEqual(caught.exception.exit_code, 7)
        self.assertEqual(caught.exception.details["transaction_id"], TRANSACTION)
        path = self.root / ".local/deployments" / TRANSACTION / "intent.json"
        self.assertTrue(path.exists())
        self.assertEqual(path.stat().st_mode & 0o777, 0o600)
        self.exchange.side_effect = None
        _, result = self.call("status", acknowledge=False)
        self.assertEqual(self.headroom.call_count, 1)
        self.assertEqual(result["state"], "REGISTERED")
        self.assertFalse(result["activation_performed"])
        self.synchronize.assert_called_once()
        args, payloads = self.exchange.call_args.args
        self.assertEqual(args, ["ssh", "pinned", deploy.COMMAND])
        self.assertEqual(set(sync.contract.decode(payloads[0])), {"schema_version", "operation", "identity", "transaction_id"})
        self.assertEqual(self.call()[1], self.call()[1])

    def test_changed_boot_stops_before_helper(self):
        self.call()
        self.exchange.reset_mock()
        self.enrolled_identity.return_value = (self.trust, {**IDENTITY, "boot_id": "4" * 8 + IDENTITY["boot_id"][8:]})
        with self.assertRaises(DevctlError) as caught:
            self.call("status")
        self.assertEqual(caught.exception.exit_code, 4)
        self.exchange.assert_not_called()

    def test_reinstalled_target_cannot_reuse_previous_registration(self):
        self.call()
        self.exchange.reset_mock()
        self.enrolled_identity.return_value=(self.trust,{**IDENTITY,"installation_uuid":"55555555-5555-4555-8555-555555555555"})
        with self.assertRaises(DevctlError) as caught:
            self.call("status")
        self.assertEqual(caught.exception.code,"DEPLOYMENT_TARGET_CHANGED")
        self.exchange.assert_not_called()

    def test_product_role_wrong_disk_and_channel_are_denied(self):
        for key, value in (("guest_role", "production"), ("disk_serial", "wrong"), ("management_channel", "other")):
            self.enrolled_identity.return_value = (self.trust, {**IDENTITY, key: value})
            with self.assertRaises(DevctlError):
                self.call()
        self.exchange.assert_not_called()
        self.synchronize.assert_not_called()

    def test_forged_receipts_never_become_success(self):
        for key, value in (("activation_performed", True), ("developer_uid", True), ("source_dirty", 1),
                           ("snapshot_digest", "f" * 64), ("helper_sha256", "not-a-hash"), ("extra", "ignored")):
            self.exchange.return_value = (0, sync.contract.canonical({**self.receipt, key: value}), b"")
            with self.assertRaises(DevctlError) as caught:
                self.call()
            self.assertEqual(caught.exception.code, "DEPLOYMENT_RECEIPT_MISMATCH")
        receipts = self.root / ".local/deployments" / TRANSACTION / "receipts"
        self.assertFalse(receipts.exists() and any(receipts.iterdir()))

    def test_post_operation_identity_change_invalidates_receipt(self):
        changed = {**IDENTITY, "current_system": "/nix/store/" + "b" * 32 + "-nixos-system-aios-dev-26.05"}
        self.enrolled_identity.side_effect = [(self.trust, IDENTITY), (self.trust, IDENTITY), (self.trust, changed)]
        with self.assertRaises(DevctlError) as caught:
            self.call()
        self.assertEqual(caught.exception.code, "DEPLOYMENT_TARGET_CHANGED")

    def test_test_and_commit_accept_only_qualified_guard_receipts(self):
        self.call()
        candidate = "/nix/store/" + "b" * 32 + "-nixos-system-aios-dev-26.05"
        built = {
            **self.receipt,
            "state": "TESTED",
            "activation_performed": True,
            "candidate_closure": candidate,
            "candidate_digest": "c" * 64,
            "build_source_digest": "d" * 64,
            "test_guard_id": "11111111-1111-4111-8111-111111111111",
            "baseline": {"running": IDENTITY["current_system"], "profile": IDENTITY["current_system"], "booted": IDENTITY["current_system"]},
            "limitations": ["Candidate test activation passed and rolled back; commit remains required."],
        }
        self.exchange.return_value = (0, sync.contract.canonical(built), b"")
        _, tested = self.call("test")
        self.assertEqual(tested["state"], "TESTED")
        self.assertTrue(Path(tested["artifact_path"]).name == "test-tested.json")

        committed_identity = {**IDENTITY, "current_system": candidate}
        committed = {
            **built,
            "state": "COMMITTED",
            "commit_guard_id": "22222222-2222-4222-8222-222222222222",
            "committed_identity": committed_identity,
            "limitations": [],
        }
        self.exchange.return_value = (0, sync.contract.canonical(committed), b"")
        self.enrolled_identity.side_effect = [(self.trust, IDENTITY), (self.trust, committed_identity)]
        _, result = self.call("commit")
        self.assertEqual(result["state"], "COMMITTED")
        self.assertEqual(result["committed_identity"], committed_identity)

    def test_test_or_commit_cannot_claim_success_without_guard_identity(self):
        self.call()
        for mode in ("test", "commit"):
            forged = {**self.receipt, "state": "TESTED", "activation_performed": True}
            self.exchange.return_value = (0, sync.contract.canonical(forged), b"")
            with self.assertRaises(DevctlError) as caught:
                self.call(mode)
            self.assertEqual(caught.exception.code, "DEPLOYMENT_RECEIPT_MISMATCH")

    def test_parser_exposes_typed_modes_without_shell_or_targets(self):
        args = parser().parse_args(["deploy", "--mode", "register", "--acknowledge-guest-root"])
        self.assertTrue(args.acknowledge_guest_root)
        for flag in ("--command", "--target", "--source-root"):
            with self.assertRaises(DevctlError):
                parser().parse_args(["deploy", "--mode", "register", flag, "untrusted"])


if __name__ == "__main__":
    unittest.main()
