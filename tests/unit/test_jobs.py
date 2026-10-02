"""Host fixtures for registered job guards; no guest builds execute here."""
import copy
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from aios_dev import jobs, sync
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError
from test_guest import EXAMPLE, IDENTITY

sys.modules["snapshot"] = sync.contract
spec = importlib.util.spec_from_file_location("aios_job_fixture", ROOT / "tools/guest/jobs.py")
controller = importlib.util.module_from_spec(spec)
spec.loader.exec_module(controller)
JOB = "44444444-4444-4444-8444-444444444444"


class JobTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="aios-job-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.root.chmod(0o700)
        self.config = VMConfig.from_data(self.root, EXAMPLE)
        self.request = {"schema_version": 1, "operation": "start", "identity": IDENTITY, "job_id": JOB,
                        "kind": "test-unit", "package": None, "source_root": "/home/dev/aios-releases", "snapshot_digest": "a" * 64}

    def test_target_mismatch_precedes_job_or_source_mutation(self):
        with patch.object(controller, "job_root") as root, patch.object(controller, "validate_start") as validate:
            code, result = controller.dispatch(self.request, lambda: {**IDENTITY, "os_id": "cachyos"})
        self.assertEqual(code, 4)
        root.assert_not_called()
        validate.assert_not_called()

    def test_arbitrary_shell_and_extra_request_fields_denied(self):
        for request in ([], None, {**self.request, "command": "sudo true"}, {**self.request, "schema_version": True}, {**self.request, "operation": "exec"}):
            with self.assertRaises(ValueError):
                controller.dispatch(request, lambda: IDENTITY)
        for kind in ("shell", "deploy-unchecked", "arbitrary-nix"):
            with self.assertRaises(ValueError):
                controller.commands(kind, Path("/home/dev/aios-releases") / ("a" * 64))

    def test_invalid_job_id_cannot_launch_or_cancel(self):
        for value in ("../other-job", "/etc/nixos", "not-uuid"):
            with self.assertRaises(DevctlError):
                jobs.identifier(value)

    def test_build_targets_use_immutable_paths_and_no_lock_update(self):
        release = Path("/home/dev/aios-releases") / ("a" * 64)
        command = controller.commands("build-packages", release)[0]
        for name in controller.PACKAGES:
            self.assertIn("path:" + str(release) + "#" + name, command)
        self.assertIn("--no-write-lock-file", command)
        self.assertIn("--no-update-lock-file", command)
        with self.assertRaises(ValueError):
            controller.commands("build-packages", release, "evil#output")

    def test_logs_remove_credentials_control_codes_and_private_key_blocks(self):
        raw = "password=fixturesecret\nAuthorization: BearerToken\n" + "-----BEGIN " + "OPENSSH PRIVATE KEY-----\nfixturesecret\n-----END " + "OPENSSH PRIVATE KEY-----\n\x1b[31mpublic\x1b[0m\x00"
        clean = controller.sanitize(raw)
        self.assertNotIn("fixturesecret", clean)
        self.assertNotIn("BearerToken", clean)
        self.assertNotIn("\x1b", clean)
        self.assertNotIn("\x00", clean)
        self.assertIn("public", clean)

    def test_cancelled_or_reused_pid_is_never_signalled(self):
        directory = self.root / JOB
        directory.mkdir(mode=0o700)
        request = {key: self.request[key] for key in ("schema_version", "identity", "job_id")}
        request["operation"] = "cancel"
        record = {"schema_version": 1, "job_id": JOB, "identity": IDENTITY, "state": "running", "worker_pid": 42, "worker_start_ticks": 123}
        controller.atomic(directory / "report.json", record)
        with patch.object(controller, "job_root", return_value=self.root), patch.object(controller, "process_matches", return_value=False), patch.object(controller.signal, "pidfd_send_signal") as send:
            code, result = controller.dispatch(request, lambda: IDENTITY)
        self.assertEqual(code, 0)
        self.assertEqual(result["state"], "interrupted")
        send.assert_not_called()

    def test_job_directory_symlink_cannot_escape(self):
        outside = self.root / "outside"
        outside.mkdir(mode=0o700)
        controller.atomic(outside / "report.json", {"fixture": True})
        link = self.root / JOB
        link.symlink_to(outside, target_is_directory=True)
        with self.assertRaises(ValueError):
            controller.read_record(link)

    def test_missing_locks_is_a_prerequisite_not_a_successful_build(self):
        with patch.object(controller, "job_root", return_value=self.root), patch.object(controller, "validate_start", return_value=(None, {})), patch.object(controller.subprocess, "Popen") as spawn:
            code, result = controller.dispatch(self.request, lambda: IDENTITY)
        self.assertEqual(code, 3)
        self.assertEqual(result["error"], "SOURCE_LOCKS_REQUIRED")
        spawn.assert_not_called()

    def test_host_identity_failure_prevents_job_transport(self):
        with patch("aios_dev.jobs.guest.enrolled_identity", side_effect=DevctlError(4, "MISMATCH", "fixture")), patch("aios_dev.jobs.sync.exchange") as exchange:
            with self.assertRaises(DevctlError):
                jobs.request(self.config, "cancel", JOB)
        exchange.assert_not_called()


if __name__ == "__main__":
    unittest.main()
