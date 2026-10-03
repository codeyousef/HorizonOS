"""Host boundary fixtures; no desktop/runtime success is established here."""
import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from aios_dev import desktop, sync
from aios_dev.cli import main
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError, ExitCode

EXAMPLE = json.loads((ROOT / "dev/vm.example.json").read_text())


class DesktopTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.config = VMConfig.from_data(Path(self.temp.name), EXAMPLE)
        self.identity = {"boot_id": "synthetic-boot-fixture"}
        self.response = {"schema_version": 1, "profile": desktop.PROFILE, "boot_id": self.identity["boot_id"], "tester_uid": 1001,
                         "session_id": "2", "processes": [{"pid": 45, "uid": 1001, "name": "kwin_wayland"}, {"pid": 46, "uid": 1001, "name": "plasmashell"}]}

    def invoke(self, arguments):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            code = main(arguments + ["--json"])
        return code, json.loads(output.getvalue())

    def test_cli_routes_only_desktop_scope(self):
        with patch("aios_dev.desktop.run", return_value=(ExitCode.SUCCESS, {"run_id": "fixture"})) as runner:
            code, value = self.invoke(["test", "--suite", "desktop"])
        self.assertEqual(code, 0)
        self.assertEqual(value["target"], "host")
        self.assertEqual(runner.call_count, 1)
        for arguments in (["test", "--suite", "desktop", "--detach"], ["test", "--suite", "unit", "--desktop-run", "fixture"],
                          ["test", "--suite", "desktop", "--provider", "system-info"], ["test", "--suite", "desktop", "--bootstrap-case", "all"]):
            with patch("aios_dev.desktop.run") as runner:
                code, _ = self.invoke(arguments)
            self.assertEqual(code, 2)
            runner.assert_not_called()

    def test_owner_identity_failure_precedes_source_or_vm_work(self):
        owner = self.config
        with patch("aios_dev.acceptance.STORAGE_ROOT", self.config.root), patch("aios_dev.guest.enrolled_identity", side_effect=DevctlError(ExitCode.TARGET_MISMATCH, "WRONG_TARGET", "fixture")), patch("aios_dev.sync.collect") as collect, patch("aios_dev.provision.create") as create:
            with self.assertRaises(DevctlError):
                desktop.prepare(owner)
        collect.assert_not_called()
        create.assert_not_called()

    def test_resume_uuid_cannot_escape_workspace(self):
        for value in ("../../root", "", None, "ABCDEF00-1111-4111-8111-111111111111"):
            with self.assertRaises(DevctlError):
                desktop.identifier(value)

    def test_unregistered_workspace_cannot_be_used(self):
        with self.assertRaises((DevctlError, FileNotFoundError)), patch("aios_dev.vm.start") as start:
            desktop.binding(self.config, "11111111-1111-4111-8111-111111111111")
        start.assert_not_called()

    def observe(self, value):
        with patch("aios_dev.guest.enrolled_identity", return_value=({"host_key_fingerprint": "fixture"}, self.identity)), patch("aios_dev.guest.ssh_arguments", return_value=["ssh", "identity"]), patch("aios_dev.sync.exchange", return_value=(0, json.dumps(value).encode(), b"")) as exchange:
            result = desktop.observe(self.config)
        self.assertEqual(exchange.call_args.args[0][-1], "/run/current-system/sw/bin/aios-desktop-test-probe")
        self.assertEqual(exchange.call_args.args[1], [b""])
        return result

    def test_read_only_transport_closes_stdin_without_sending_input(self):
        # Real local pipes qualify transport only; no guest/SSH is contacted.
        status, output, _ = sync.exchange([sys.executable, "-c", "import sys; data=sys.stdin.buffer.read(); print(len(data))"], [b""], timeout=5)
        self.assertEqual(status, 0)
        self.assertEqual(output, b"0\n")

    def test_fixed_probe_requires_current_boot_and_same_uid_processes(self):
        self.assertEqual(self.observe(self.response)["desktop"], self.response)
        for value in ({**self.response, "boot_id": "old"}, {**self.response, "profile": "production"}, {**self.response, "schema_version": True},
                      {**self.response, "processes": [{"pid": 45, "uid": 0, "name": "kwin_wayland"}, self.response["processes"][1]]},
                      {**self.response, "processes": []}, {**self.response, "extra": "untrusted"}):
            with self.assertRaises(DevctlError) as caught:
                self.observe(value)
            self.assertEqual(caught.exception.exit_code, ExitCode.VERIFICATION_FAILURE)

    def test_probe_identity_failure_precedes_ssh(self):
        with patch("aios_dev.guest.enrolled_identity", side_effect=DevctlError(ExitCode.TARGET_MISMATCH, "WRONG_TARGET", "fixture")), patch("aios_dev.sync.exchange") as exchange:
            with self.assertRaises(DevctlError):
                desktop.observe(self.config)
        exchange.assert_not_called()


if __name__ == "__main__":
    unittest.main()
