"""Filesystem/authority fixtures; not installed-root or activation acceptance."""
import copy
import hashlib
import importlib.util
import os
from pathlib import Path
import shutil
import sys
import tempfile
import types
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from aios_dev import sync
from test_guest import IDENTITY
sys.modules["snapshot"] = sync.contract
spec = importlib.util.spec_from_file_location("developer_boundary_fixture", ROOT / "tools/guest/dev_deploy.py")
deploy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deploy)

CONFIG = {"schema_version": 1, "enabled": True, "expected_vm_uuid": IDENTITY["dmi_uuid"],
          "expected_installation_uuid": IDENTITY["installation_uuid"], "developer_user": "dev",
          "disk_serial": IDENTITY["disk_serial"], "management_channel": IDENTITY["management_channel"]}
TRANSACTION = "99999999-9999-4999-8999-999999999999"


class DeveloperBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="aios-dev-boundary-")
        self.root = Path(self.temp.name)
        self.addCleanup(self.cleanup)
        self.releases = self.root / "releases"
        self.releases.mkdir(mode=0o700)
        self.parent = self.root / "aios"
        self.parent.mkdir(mode=0o755)
        self.state = self.parent / "development"
        files = {"Cargo.lock": b"# fixture lock\n", "flake.nix": b"{ outputs = _: {}; }\n", "src/main.rs": b"fn main() {}\n"}
        self.manifest = {"schema_version": 1, "git_head": "1" * 40, "dirty": True, "files": [
            {"path": name, "size": len(data), "sha256": hashlib.sha256(data).hexdigest(), "mode": 0o644}
            for name, data in sorted(files.items())]}
        self.digest = sync.contract.validate_manifest(self.manifest)
        self.release = self.releases / self.digest
        self.release.mkdir(mode=0o700)
        for name, data in files.items():
            path = self.release / name
            path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            path.write_bytes(data)
        (self.release / sync.contract.MANIFEST).write_bytes(sync.contract.canonical(self.manifest))
        deploy.seal_stage(self.release)
        self.request = {"schema_version": 1, "operation": "register", "transaction_id": TRANSACTION,
                        "identity": copy.deepcopy(IDENTITY), "snapshot_digest": self.digest, "authority": deploy.AUTHORITY}
        self.inject = dict(config_reader=lambda: CONFIG, identity_reader=lambda: IDENTITY, caller_reader=lambda: os.getuid(),
                           state=self.state, releases=self.releases, owner_uid=os.getuid())

    def cleanup(self):
        for parent, _, _ in os.walk(self.root):
            Path(parent).chmod(0o700)
        self.temp.cleanup()

    def call(self, request=None, **overrides):
        return deploy.dispatch(request or self.request, **{**self.inject, **overrides})

    def test_strict_schema_denies_shell_paths_duplicates_booleans_and_authority_substitution(self):
        for request in ([], {}, {**self.request, "command": "true"}, {**self.request, "target": "/etc/nixos"},
                        {**self.request, "source_root": "/home/dev"}, {**self.request, "schema_version": True},
                        {**self.request, "operation": "shell"}, {**self.request, "authority": "product"},
                        {**self.request, "snapshot_digest": "../escape"}, {**self.request, "transaction_id": "../escape"}):
            with self.assertRaises(ValueError):
                deploy.validate_request(request)
        for payload in (b'{"schema_version":1,"schema_version":1}', b'{"x":NaN}'):
            with self.assertRaises(ValueError):
                sync.contract.decode(payload)

    def test_product_caller_is_denied_before_config_identity_and_state(self):
        def denial():
            raise deploy.Denial(5, "DEVELOPER_AUTHORITY_REQUIRED")
        with patch.object(deploy, "state_directory") as state, patch.object(deploy, "read_config") as config:
            with self.assertRaises(deploy.Denial) as raised:
                self.call(caller_reader=denial, config_reader=config)
        self.assertEqual(raised.exception.code, 5)
        config.assert_not_called()
        state.assert_not_called()

    def test_forged_sudo_environment_does_not_grant_nonroot_authority(self):
        with patch.object(deploy.os, "getuid", return_value=1000), patch.dict(os.environ, {"SUDO_USER": "dev", "SUDO_UID": "1000", "SUDO_GID": "100"}):
            with self.assertRaises(deploy.Denial):
                deploy.caller_uid()

    def test_host_or_unsupported_virtualization_is_not_a_development_vm(self):
        for status, virtualization in ((1, b"none"), (0, b"vmware"), (0, b"kvm\nextra")):
            result = deploy.subprocess.CompletedProcess([], status, virtualization, b"")
            with patch.object(deploy.subprocess, "run", return_value=result), patch.object(deploy.snapshot, "identity") as identity:
                with self.assertRaises(deploy.Denial) as raised:
                    deploy.root_identity()
            self.assertEqual(raised.exception.label, "DEVELOPMENT_VM_REQUIRED")
            identity.assert_not_called()

    def test_root_dmi_must_match_the_published_identity(self):
        result = deploy.subprocess.CompletedProcess([], 0, b"kvm\n", b"")
        with patch.object(deploy.subprocess, "run", return_value=result), patch.object(deploy.snapshot, "identity", return_value=IDENTITY), patch.object(deploy.Path, "read_text", return_value="different"):
            with self.assertRaises(deploy.Denial):
                deploy.root_identity()

    def test_each_target_component_is_checked_before_writes(self):
        for key, value in (("os_id", "cachyos"), ("guest_role", "production"), ("disk_serial", "WRONG"),
                           ("management_channel", "other"), ("dmi_uuid", "a" * 8 + IDENTITY["dmi_uuid"][8:]),
                           ("installation_uuid", "b" * 8 + IDENTITY["installation_uuid"][8:]),
                           ("boot_id", "c" * 8 + IDENTITY["boot_id"][8:])):
            actual = {**IDENTITY, key: value}
            with self.assertRaises(deploy.Denial):
                self.call(identity_reader=lambda: actual)
            self.assertFalse(self.state.exists())

    def test_preexisting_state_still_checks_target_before_lookup(self):
        self.state.mkdir(mode=0o700)
        status = {key: self.request[key] for key in ("schema_version", "identity", "transaction_id")}
        status["operation"] = "status"
        wrong = {**IDENTITY, "os_id": "cachyos"}
        with self.assertRaises(deploy.Denial) as raised:
            self.call(status, identity_reader=lambda: wrong)
        self.assertEqual(raised.exception.label, "DEVELOPMENT_TARGET_MISMATCH")
        self.assertEqual(list(self.state.iterdir()), [])

    def test_broker_only_shared_ledger_has_no_guard_state(self):
        path = self.root / "ledger.sqlite"
        connection = deploy.sqlite3.connect(path)
        connection.execute("CREATE TABLE broker_schema(version INTEGER NOT NULL)")
        connection.commit()
        connection.close()
        path.chmod(0o600)
        self.assertIsNone(deploy.guard_state(TRANSACTION, path, os.getuid()))

    def test_source_becomes_separate_immutable_verified_candidate_and_durable_receipt(self):
        receipt = self.call()
        candidate = self.state / "releases" / self.digest
        self.assertEqual(receipt["state"], "REGISTERED")
        self.assertFalse(receipt["activation_performed"])
        self.assertTrue(receipt["source_dirty"])
        self.assertEqual(candidate.stat().st_mode & 0o777, 0o555)
        self.assertEqual((self.state / "registrations.sqlite").stat().st_mode & 0o777, 0o600)
        for item in self.manifest["files"]:
            src, dst = self.release / item["path"], candidate / item["path"]
            self.assertNotEqual(src.stat().st_ino, dst.stat().st_ino)
            self.assertEqual(dst.read_bytes(), src.read_bytes())
        # Later developer source changes cannot change the registered candidate.
        source = self.release / "flake.nix"
        source.chmod(0o644)
        source.write_bytes(b"changed developer code")
        self.assertEqual(self.call(), receipt)
        status = {key: self.request[key] for key in ("schema_version", "identity", "transaction_id")}
        status["operation"] = "status"
        self.assertEqual(self.call(status), receipt)

    def test_replay_with_changed_source_or_owner_is_denied(self):
        self.call()
        with self.assertRaises(deploy.Denial):
            self.call({**self.request, "snapshot_digest": "f" * 64})
        with self.assertRaises(deploy.Denial):
            self.call(caller_reader=lambda: os.getuid() + 1)

    def test_corrupted_registered_copy_is_not_reblessed_on_replay(self):
        self.call()
        candidate = self.state / "releases" / self.digest / "flake.nix"
        candidate.chmod(0o644)
        candidate.write_bytes(b"corrupted")
        with self.assertRaises(ValueError):
            self.call()

    def test_release_symlink_and_hardlinked_file_and_unlisted_entries_are_denied(self):
        for kind in ("symlink", "hardlink", "unlisted"):
            with self.subTest(kind=kind):
                file = self.release / "flake.nix"
                self.release.chmod(0o700)
                if kind == "symlink":
                    old = file.read_bytes()
                    file.unlink()
                    file.symlink_to("/etc/passwd")
                elif kind == "hardlink":
                    os.link(file, self.root / "linked-file")
                else:
                    (self.release / "unlisted").write_bytes(b"unexpected")
                self.release.chmod(0o555)
                with self.assertRaises((ValueError, OSError)):
                    self.call()
                self.release.chmod(0o700)
                if kind == "symlink":
                    file.unlink()
                    file.write_bytes(old)
                    file.chmod(0o444)
                elif kind == "hardlink":
                    (self.root / "linked-file").unlink()
                else:
                    (self.release / "unlisted").unlink()
                self.release.chmod(0o555)
                self.assertFalse((self.state / "releases" / self.digest).exists())

    def test_changed_bytes_during_copy_fail_and_unique_stage_is_cleaned(self):
        original = deploy.read_source
        calls = 0
        def raced(fd, name, uid):
            nonlocal calls
            if name == "flake.nix":
                calls += 1
                if calls == 2:
                    file = self.release / name
                    file.chmod(0o644)
                    file.write_bytes(b"changed during copy")
                    file.chmod(0o444)
            return original(fd, name, uid)
        with patch.object(deploy, "read_source", side_effect=raced):
            with self.assertRaises(ValueError):
                self.call()
        self.assertEqual(list((self.state / "releases").iterdir()), [])

    def test_state_or_ledger_symlink_is_denied_without_outside_writes(self):
        outside = self.root / "outside"
        outside.mkdir(mode=0o700)
        self.state.symlink_to(outside, target_is_directory=True)
        with self.assertRaises(ValueError):
            self.call()
        self.assertEqual(list(outside.iterdir()), [])
        self.state.unlink()
        self.state.mkdir(mode=0o700)
        (self.state / "registrations.sqlite").symlink_to(self.root / "outside.sqlite")
        with self.assertRaises(ValueError):
            self.call()
        self.assertFalse((self.root / "outside.sqlite").exists())

    def test_target_change_before_commit_leaves_no_registration(self):
        actual = iter([IDENTITY, {**IDENTITY, "disk_serial": "WRONG"}])
        with self.assertRaises(deploy.Denial):
            self.call(identity_reader=lambda: next(actual))
        database = deploy.sqlite3.connect(self.state / "registrations.sqlite")
        try:
            self.assertEqual(database.execute("SELECT COUNT(*) FROM registrations").fetchone()[0], 0)
        finally:
            database.close()

    def test_test_and_commit_require_the_registered_exact_candidate_and_independent_guard(self):
        self.call()
        candidate = "/nix/store/" + "a" * 32 + "-nixos-system-aios-dev-test"
        baseline = {
            "running": IDENTITY["current_system"],
            "profile": IDENTITY["current_system"],
            "booted": IDENTITY["current_system"],
        }
        built = {
            "schema_version": 1,
            "snapshot_digest": self.digest,
            "build_source_digest": "b" * 64,
            "candidate_closure": candidate,
            "candidate_digest": hashlib.sha256(candidate.encode()).hexdigest(),
            "baseline": baseline,
        }
        operations = []
        current = copy.deepcopy(IDENTITY)

        def identity():
            return copy.deepcopy(current)

        def handoff(_state, prior, receipt, operation, _actual):
            operations.append(("handoff", operation, receipt["candidate_closure"]))
            return deploy.guard_id(prior["transaction_id"], operation)

        def complete(_identifier, operation, _receipt):
            operations.append(("guard", operation))
            if operation == "commit":
                current["current_system"] = candidate

        test_request = {**self.request, "operation": "test"}
        with patch.object(deploy, "build_candidate", return_value=built), \
             patch.object(deploy, "guard_state", return_value=None), \
             patch.object(deploy, "write_handoff", side_effect=handoff), \
             patch.object(deploy, "complete_guard", side_effect=complete):
            tested = self.call(test_request, identity_reader=identity)
        self.assertEqual(tested["state"], "TESTED")
        self.assertTrue(tested["activation_performed"])
        self.assertEqual(current, IDENTITY)
        self.assertEqual(operations, [("handoff", "test", candidate), ("guard", "test")])

        fake_build = types.SimpleNamespace(pointers=lambda: baseline if current["current_system"] == IDENTITY["current_system"] else {
            "running": candidate, "profile": candidate, "booted": baseline["booted"]})
        commit_request = {**self.request, "operation": "commit"}
        with patch.dict(sys.modules, {"build_system": fake_build}), \
             patch.object(deploy, "guard_state", return_value=None), \
             patch.object(deploy, "write_handoff", side_effect=handoff), \
             patch.object(deploy, "complete_guard", side_effect=complete):
            committed = self.call(commit_request, identity_reader=identity)
        self.assertEqual(committed["state"], "COMMITTED")
        self.assertEqual(committed["committed_identity"]["current_system"], candidate)
        self.assertEqual(operations[-2:], [("handoff", "commit", candidate), ("guard", "commit")])

    def test_commit_before_successful_test_is_denied_without_guard_start(self):
        self.call()
        with patch.object(deploy, "complete_guard") as guard:
            with self.assertRaises(deploy.Denial) as raised:
                self.call({**self.request, "operation": "commit"})
        self.assertEqual(raised.exception.label, "DEPLOYMENT_STATE_INVALID")
        guard.assert_not_called()


if __name__ == "__main__":
    unittest.main()
