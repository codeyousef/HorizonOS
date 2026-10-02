"""Host-only adversarial source/receiver fixtures, not guest acceptance."""
from concurrent.futures import ThreadPoolExecutor
import copy
import hashlib
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
from aios_dev.errors import DevctlError
from aios_dev.sync import collect, contract, synchronize, transfer
from aios_dev.config import VMConfig
from test_guest import EXAMPLE, IDENTITY


class SourceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="aios-source-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Source fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        (self.root / "flake.nix").write_text("fixture public source")
        (self.root / ".gitignore").write_text("ignored.txt\n")
        self.git("add", ".")
        self.git("commit", "-qm", "public fixture")

    def git(self, *args):
        subprocess.run(["git", "-C", str(self.root), *args], check=True, capture_output=True)

    def test_clean_dirty_new_deleted_and_ignored_are_distinguished(self):
        clean, digest, _ = collect(self.root)
        self.assertFalse(clean["dirty"])
        (self.root / "new.txt").write_text("explicit nonignored project file")
        (self.root / "ignored.txt").write_text("not source")
        (self.root / "flake.nix").unlink()
        dirty, changed, _ = collect(self.root)
        self.assertTrue(dirty["dirty"])
        self.assertNotEqual(changed, digest)
        self.assertEqual(dirty["git_head"], clean["git_head"])
        self.assertEqual([f["path"] for f in dirty["files"]], [".gitignore", "new.txt"])

    def test_credential_and_artifact_paths_excluded_even_when_tracked(self):
        for name in (".local/private.txt", ".ssh/id_ed25519", "model.gguf", "build/output", "secrets/token.txt", ".env", "id_rsa", "image.qcow2", "secrets.yaml", "credentials.toml", "api_keys.json", "models/weights.bin"):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("PRIVATE FIXTURE")
        self.git("add", "-f", ".")
        manifest, _, contents = collect(self.root)
        self.assertEqual([f["path"] for f in manifest["files"]], [".gitignore", "flake.nix"])
        self.assertNotIn(b"PRIVATE FIXTURE", contents)

    def test_filemode_changes_are_dirty_even_when_git_ignores_them(self):
        self.git("config", "core.filemode", "false")
        (self.root / "flake.nix").chmod(0o755)
        manifest, _, _ = collect(self.root)
        self.assertTrue(manifest["dirty"])
        self.assertEqual(manifest["files"][1]["mode"], 0o755)

    def test_special_symlink_and_unsafe_mode_denied(self):
        path = self.root / "new.txt"
        for setup in (lambda: path.symlink_to(self.root / "flake.nix"), lambda: path.symlink_to("/etc/passwd")):
            setup()
            with self.assertRaises(DevctlError):
                collect(self.root)
            path.unlink()
        path.write_text("tracked fixture")
        self.git("add", "new.txt")
        path.unlink()
        os.mkfifo(path)
        with self.assertRaises(DevctlError):
            collect(self.root)
        path.unlink()
        path.write_text("public")
        for mode in (0o600, 0o666, 0o4755):
            path.chmod(mode)
            with self.assertRaises(DevctlError):
                collect(self.root)

    def test_parent_symlink_and_private_key_in_public_file_denied(self):
        (self.root / "linked").symlink_to(self.root, target_is_directory=True)
        self.git("add", "linked")
        with self.assertRaises(DevctlError):
            collect(self.root)
        self.git("rm", "--cached", "linked")
        (self.root / "linked").unlink()
        (self.root / "new.txt").write_text("-----BEGIN " + "OPENSSH PRIVATE KEY-----")
        with self.assertRaises(DevctlError):
            collect(self.root)

    def test_no_ssh_mutation_before_identity_verification(self):
        config = VMConfig.from_data(self.root, EXAMPLE)
        with patch("aios_dev.sync.guest.enrolled_identity", side_effect=DevctlError(4, "MISMATCH", "fixture")), patch("aios_dev.sync.transfer") as send:
            with self.assertRaises(DevctlError):
                synchronize(config)
        send.assert_not_called()

    def test_manifest_rejects_unsafe_paths_modes_duplicates_and_provenance(self):
        manifest, _, _ = collect(self.root)
        for name in ("/etc/passwd", "../escape", "a/../escape", "a//b", "./a", "a\\b", "a\nb", ".local/secret", contract.MANIFEST):
            bad = copy.deepcopy(manifest)
            bad["files"][0]["path"] = name
            with self.subTest(name=name), self.assertRaises(ValueError):
                contract.validate_manifest(bad)
        for field, value in (("mode", 0o777), ("mode", True), ("size", -1), ("sha256", "bad")):
            bad = copy.deepcopy(manifest)
            bad["files"][0][field] = value
            with self.assertRaises(ValueError):
                contract.validate_manifest(bad)
        for field, value in (("schema_version", True), ("dirty", "false"), ("git_head", "unknown")):
            with self.assertRaises(ValueError):
                contract.validate_manifest({**manifest, field: value})
        bad = copy.deepcopy(manifest)
        bad["files"].append(bad["files"][0])
        with self.assertRaises(ValueError):
            contract.validate_manifest(bad)
        entry = manifest["files"][0]
        with self.assertRaises(ValueError):
            contract.validate_manifest({**manifest, "files": [{**entry, "path": "a"}, {**entry, "path": "a/b"}]})


class ReceiverTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="aios-receiver-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.root.chmod(0o700)
        self.data = b"public source"
        self.manifest = {"schema_version": 1, "git_head": "a" * 40, "dirty": True,
                         "files": [{"path": "nix/nested/source.nix", "mode": 0o644, "size": len(self.data), "sha256": hashlib.sha256(self.data).hexdigest()}]}
        self.digest = contract.validate_manifest(self.manifest)
        self.request = {"schema_version": 1, "operation": "publish-source", "identity": IDENTITY,
                        "source_root": "/home/dev/aios-releases", "manifest": self.manifest, "digest": self.digest}

    def wire(self, request=None, data=None):
        header = contract.canonical(request or self.request)
        return io.BytesIO(f"{len(header):08d}".encode() + header + (self.data if data is None else data))

    def receive(self, **kwargs):
        return contract.receive(self.wire(**kwargs), lambda: IDENTITY)

    def test_publish_and_reuse_verify_readonly_content_and_manifest(self):
        with patch.object(contract, "release_root", return_value=self.root):
            first = self.receive()
            second = self.receive()
        self.assertEqual(first["guest_digest"], self.digest)
        self.assertFalse(first["reused"])
        self.assertTrue(second["reused"])
        self.assertEqual((self.root / self.digest / "nix/nested/source.nix").stat().st_mode & 0o777, 0o444)
        self.assertEqual(list(self.root.glob(".incoming-*")), [])

    def test_wrong_target_stops_before_release_directory_creation(self):
        with patch.object(contract, "release_root") as mkdir:
            with self.assertRaises(ValueError):
                self.receive(request={**self.request, "identity": {**IDENTITY, "os_id": "cachyos"}})
        mkdir.assert_not_called()

    def test_corruption_truncation_and_trailing_data_never_publish(self):
        with patch.object(contract, "release_root", return_value=self.root):
            for data in (b"bad source!!!", self.data[:-1], self.data + b"extra"):
                with self.assertRaises(ValueError):
                    self.receive(data=data)
                self.assertEqual(list(self.root.iterdir()), [])

    def test_existing_release_tamper_is_rejected_and_retained(self):
        with patch.object(contract, "release_root", return_value=self.root):
            self.receive()
            source = self.root / self.digest / "nix/nested/source.nix"
            source.chmod(0o644)
            source.write_text("tampered")
            with self.assertRaises(ValueError):
                self.receive()
        self.assertEqual(source.read_text(), "tampered")
        self.assertEqual(list(self.root.glob(".incoming-*")), [])

    def test_extra_empty_directory_is_not_verified_as_the_same_source(self):
        with patch.object(contract, "release_root", return_value=self.root):
            self.receive()
            published = self.root / self.digest
            published.chmod(0o755)
            (published / "extra").mkdir(mode=0o555)
            published.chmod(0o555)
            with self.assertRaises(ValueError):
                self.receive()

    def test_concurrent_same_and_distinct_snapshots_publish_safely(self):
        different = {**self.manifest, "dirty": False}
        other = {**self.request, "manifest": different, "digest": contract.validate_manifest(different)}
        with patch.object(contract, "release_root", return_value=self.root), ThreadPoolExecutor(max_workers=3) as pool:
            futures = [pool.submit(self.receive), pool.submit(self.receive), pool.submit(self.receive, request=other)]
            results = [future.result() for future in futures]
        self.assertEqual({r["snapshot_digest"] for r in results}, {self.digest, other["digest"]})
        self.assertEqual(sorted(p.name for p in self.root.iterdir()), sorted([self.digest, other["digest"]]))

    def test_duplicate_and_oversized_header_rejected(self):
        for wire in (b"99999999", b"00000031" + b'{"schema_version":1,"schema_version":1}'):
            with self.assertRaises(ValueError):
                contract.receive(io.BytesIO(wire), lambda: IDENTITY)


if __name__ == "__main__":
    unittest.main()
