"""Host Python fixtures for pure development candidate preparation only."""
import base64
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/guest"))
from test_guest import IDENTITY
import snapshot

spec = importlib.util.spec_from_file_location("system_build_fixture", ROOT / "tools/guest/build_system.py")
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)
KEY = "ssh-ed25519 " + base64.b64encode(b"\x00\x00\x00\x0bssh-ed25519\x00\x00\x00\x20" + b"k" * 32).decode()


class SystemCandidateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="aios-system-candidate-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.release = self.root / "release"
        self.destination = self.root / "candidate"
        self.enrolled = builder.enrollment(IDENTITY, KEY)

    def publish(self, additional=None):
        files = {"flake.nix": b"{ outputs = _: {}; }", "flake.lock": b"{}", "Cargo.lock": b"version = 4\n",
                 "nix/machines/aios-dev/default.nix": b"{}"}
        files.update(additional or {})
        self.release.mkdir(mode=0o700)
        rows = []
        for name, data in sorted(files.items()):
            path = self.release / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            path.chmod(0o444)
            rows.append({"path": name, "size": len(data), "mode": 0o644, "sha256": hashlib.sha256(data).hexdigest()})
        manifest = {"schema_version": 1, "git_head": "a" * 40, "dirty": False, "files": rows}
        (self.release / snapshot.MANIFEST).write_bytes(snapshot.canonical(manifest))
        (self.release / snapshot.MANIFEST).chmod(0o444)
        for parent, _, _ in os.walk(self.release):
            Path(parent).chmod(0o555)
        return manifest

    def test_candidate_binds_public_enrollment_and_freezes_all_bytes(self):
        manifest = self.publish()
        candidate, digest = builder.prepare(self.release, self.destination, manifest, self.enrolled)
        self.assertEqual(digest, snapshot.validate_manifest(candidate))
        self.assertEqual(json.loads((self.destination / builder.ENROLLMENT).read_text()), self.enrolled)
        self.assertTrue(candidate["dirty"])
        snapshot.verify_tree(self.destination, candidate, published=True)
        snapshot.verify_tree(self.release, manifest, published=True)
        self.assertNotEqual(snapshot.validate_manifest(manifest), digest)

    def test_enrollment_cannot_be_overridden_by_source(self):
        manifest = self.publish({builder.ENROLLMENT: snapshot.canonical(self.enrolled)})
        with self.assertRaises(ValueError):
            builder.prepare(self.release, self.destination, manifest, self.enrolled)
        self.assertFalse(self.destination.exists())

    def test_existing_candidate_is_never_overwritten(self):
        manifest = self.publish()
        self.destination.mkdir()
        sentinel = self.destination / "sentinel"
        sentinel.write_text("preserve")
        with self.assertRaises(FileExistsError):
            builder.prepare(self.release, self.destination, manifest, self.enrolled)
        self.assertEqual(sentinel.read_text(), "preserve")

    def test_changed_source_after_initial_verification_is_rejected_and_cleaned(self):
        manifest = self.publish()
        read = snapshot.read_regular
        def changed(root, name, **kwargs):
            data, mode = read(root, name, **kwargs)
            return (data + b"changed", mode) if name == "flake.nix" else (data, mode)
        with patch.object(snapshot, "verify_tree"), patch.object(snapshot, "read_regular", side_effect=changed):
            with self.assertRaises(ValueError):
                builder.prepare(self.release, self.destination, manifest, self.enrolled)
        self.assertFalse(self.destination.exists())

    def test_symlink_source_never_enters_candidate(self):
        manifest = self.publish()
        self.release.chmod(0o700)
        path = self.release / "flake.nix"
        path.unlink()
        path.symlink_to(self.root / "outside")
        self.release.chmod(0o555)
        with self.assertRaises((ValueError, OSError)):
            builder.prepare(self.release, self.destination, manifest, self.enrolled)
        self.assertFalse(self.destination.exists())

    def test_private_key_source_never_enters_candidate(self):
        manifest = self.publish({"untrusted.txt": ("-----BEGIN " + "OPENSSH PRIVATE KEY-----\nfixture").encode()})
        with self.assertRaises(ValueError):
            builder.prepare(self.release, self.destination, manifest, self.enrolled)
        self.assertFalse(self.destination.exists())

    def test_foreign_role_os_disk_or_management_identity_is_rejected(self):
        for key, value in (("os_id", "cachyos"), ("guest_role", "production"), ("disk_serial", "foreign"),
                           ("management_channel", "local-product"), ("dmi_uuid", "not-a-uuid")):
            with self.assertRaises(ValueError):
                builder.enrollment({**IDENTITY, key: value}, KEY)

    def test_malformed_options_multiple_keys_or_private_material_are_not_public_enrollment(self):
        for key in ("command=true " + KEY, "ssh-rsa AAAA", "ssh-ed25519 AAAA", KEY + "\n" + KEY,
                    "-----BEGIN " + "OPENSSH PRIVATE KEY-----"):
            with self.assertRaises(ValueError):
                builder.public_key(key)
        self.assertEqual(builder.public_key(KEY + " public-comment"), KEY)

    def test_installation_change_alters_candidate_digest(self):
        manifest = self.publish()
        _, first = builder.prepare(self.release, self.destination, manifest, self.enrolled)
        changed = {**self.enrolled, "installation_uuid": "55555555-5555-4555-8555-555555555555"}
        _, second = builder.prepare(self.release, self.root / "second", manifest, changed)
        self.assertNotEqual(first, second)

    def test_fixed_build_command_enforces_purity_locked_inputs_and_registered_output(self):
        args = builder.build_arguments(Path("/home/dev/candidate"), Path("/home/dev/output-root"))
        self.assertIn("--no-update-lock-file", args)
        self.assertIn("--no-write-lock-file", args)
        self.assertNotIn("--impure", args)
        self.assertIn("pure-eval", args)
        self.assertIn("allow-import-from-derivation", args)
        self.assertEqual(args[-1], "path:/home/dev/candidate#nixosConfigurations.aios-dev.config.system.build.toplevel")

    def approval(self):
        executor = "/nix/store/" + "a"*32 + "-executor"
        polkit = "/nix/store/" + "b"*32 + "-polkit"
        record = {"schema_version":1,"polkit_uid":26,"polkit_package":polkit,"policy_path":executor+"/share/aios/system-approval.json",
                  "action_path":executor+"/share/polkit-1/actions/org.aios.executor.policy","policy_sha256":"1"*64,"action_sha256":"2"*64}
        manifest = {"files":[{"path":"crates/aios-exec/policy/system-approval.json","sha256":"1"*64},
                              {"path":"crates/aios-exec/policy/org.aios.executor.policy","sha256":"2"*64}]}
        return record, manifest, executor, polkit

    def test_built_approval_policy_binds_exact_executor_and_frozen_files(self):
        record, manifest, executor, polkit = self.approval()
        self.assertEqual(builder.approval_paths(record, manifest), (executor,polkit))
        for field, value in (("policy_sha256","3"*64),("action_sha256","3"*64),("action_path",polkit+"/share/polkit-1/actions/org.aios.executor.policy"),
                             ("policy_path","/tmp/client.json"),("polkit_package",polkit+"/../other")):
            with self.subTest(field=field), self.assertRaises(ValueError):
                builder.approval_paths({**record,field:value},manifest)

    def test_built_approval_policy_denies_forged_uid_fields_and_missing_source(self):
        record, manifest, _, _ = self.approval()
        for value in (0,True,-1,2**32-1,"26"):
            with self.subTest(uid=value), self.assertRaises(ValueError):
                builder.approval_paths({**record,"polkit_uid":value},manifest)
        with self.assertRaises(ValueError):builder.approval_paths({**record,"approved":True},manifest)
        with self.assertRaises(ValueError):builder.approval_paths(record,{"files":[]})


if __name__ == "__main__":
    unittest.main()
