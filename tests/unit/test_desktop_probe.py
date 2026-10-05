"""Fixtures for the disposable desktop probe's session enumeration race."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
from types import SimpleNamespace
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "desktop_probe_fixture", Path(__file__).resolve().parents[2] / "tools/guest/desktop_probe.py")
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class DesktopProbeSessions(unittest.TestCase):
    def run_probe(self, listing, *, vanished=False, wrong_owner=False):
        inspected = []
        def command(arguments):
            if arguments[1] == "list-sessions":
                return listing
            if arguments[1] == "show-session":
                inspected.append(arguments[2])
                if arguments[2] != "2":
                    raise AssertionError("Unrelated disappearing SSH session inspected")
                if vanished:
                    raise subprocess.CalledProcessError(1, arguments)
                return "\n".join(["User=" + ("1000" if wrong_owner else "1001"),
                    "Name=tester", "Type=wayland", "Class=user", "Active=yes", "Remote=no", "State=active"])
            if arguments[0].endswith("/ps"):
                return "938 1001 /nix/store/fixture-kwin_wayland/bin/kwin_wayland\n1490 1001 /nix/store/fixture-plasmashell/bin/plasmashell\n"
            raise AssertionError(arguments)
        def read_text(path):
            if str(path) == "/etc/aios/desktop-test-profile":
                return "synthetic-disposable-plasma-wayland-v1"
            if str(path) == "/proc/sys/kernel/random/boot_id":
                return "00000000-0000-4000-8000-000000000000"
            raise AssertionError(path)
        output = io.StringIO()
        with patch.object(probe, "command", side_effect=command), \
             patch.object(probe.pwd, "getpwnam", return_value=SimpleNamespace(pw_uid=1001)), \
             patch.object(Path, "read_text", autospec=True, side_effect=read_text), \
             patch.object(Path, "resolve", autospec=True,
                          side_effect=lambda path, **_: Path("/nix/store/fixture-" + path.name + "/bin/" + path.name)), \
             contextlib.redirect_stdout(output):
            probe.main()
        return json.loads(output.getvalue()), inspected

    def test_unrelated_ssh_logout_does_not_invalidate_target_desktop(self):
        result, inspected = self.run_probe("168 1000 dev - 1200 user pts/0\n2 1001 tester seat0 938 user tty2\n1 0 root - 1 manager -\n")
        self.assertEqual(inspected, ["2"])
        self.assertEqual(result["session_id"], "2")
        self.assertEqual(result["tester_uid"], 1001)

    def test_missing_or_changed_target_is_never_healthy(self):
        with self.assertRaises(ValueError):
            self.run_probe("168 1000 dev - 1200 user pts/0\n")
        with self.assertRaises(ValueError):
            self.run_probe("2 1001 tester seat0 938 user tty2\n", wrong_owner=True)
        with self.assertRaises(subprocess.CalledProcessError):
            self.run_probe("2 1001 tester seat0 938 user tty2\n", vanished=True)
