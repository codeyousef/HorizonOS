"""Host-only tests. Fake identity data never establishes a trusted guest."""
import contextlib
import copy
import io
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from aios_dev.cli import main
from aios_dev.config import VMConfig, load_config, read_json
from aios_dev.doctor import find_firmware, host_report, memory_info, qemu_processes, ssh_file_permissions, tcp_probe, tool_info
from aios_dev.errors import DevctlError, ExitCode

EXAMPLE = json.loads((ROOT / "dev/vm.example.json").read_text())


class ConfigurationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.data = copy.deepcopy(EXAMPLE)

    def reject(self, data):
        with self.assertRaises(DevctlError) as caught:
            VMConfig.from_data(self.root, data)
        self.assertEqual(caught.exception.exit_code, ExitCode.INVALID_INPUT)

    def test_example_does_not_create_local_state(self):
        (self.root / "dev").mkdir()
        (self.root / "dev/vm.example.json").write_text(json.dumps(self.data))
        config = load_config(self.root)
        self.assertFalse(config.configured)
        self.assertEqual(config.paths["disk_image"], self.root / ".local/vm/root.qcow2")
        self.assertFalse((self.root / ".local").exists())

    def test_unknown_missing_and_boolean_integer_rejected(self):
        for data in ({**self.data, "vcpus": True}, {**self.data, "extra": 1},
                     {k: v for k, v in self.data.items() if k != "disk_gib"}, []):
            with self.subTest(data=data):
                self.reject(data)

    def test_bad_field_types_do_not_crash(self):
        for key in self.data:
            for value in (None, [], {}, True):
                with self.subTest(key=key, value=value):
                    self.reject({**self.data, key: value})

    def test_traversal_absolute_and_wrong_directory(self):
        for path in ("/dev/sda", ".local/vm/../../etc/x", ".local/ssh/disk", ".local/vm", ""):
            with self.subTest(path=path):
                self.reject({**self.data, "disk_image": path})

    def test_runtime_directory_and_file_symlink_escapes(self):
        (self.root / ".local").mkdir()
        outside = self.root / "outside"
        outside.mkdir()
        (self.root / ".local/vm").symlink_to(outside, target_is_directory=True)
        # A sibling outside the allowed VM directory is still forbidden.
        self.reject(self.data)
        (self.root / ".local/vm").unlink()
        (self.root / ".local/vm").mkdir()
        (self.root / ".local/vm/root.qcow2").symlink_to(outside / "disk")
        self.reject(self.data)

    def test_local_directory_symlink_outside_workspace(self):
        with tempfile.TemporaryDirectory() as outside:
            (self.root / ".local").symlink_to(outside, target_is_directory=True)
            with self.assertRaises(DevctlError):
                load_config(self.root)

    def test_duplicate_nonfinite_oversized_and_special_json(self):
        path = self.root / "config.json"
        for text in ('{"name":1,"name":2}', '{"n":NaN}', '{"n":Infinity}', " " * 65537, "{"):
            with self.subTest(text=text[:32]):
                path.write_text(text)
                with self.assertRaises(DevctlError):
                    read_json(path)
        path.unlink()
        os.mkfifo(path)
        with self.assertRaises(DevctlError):
            read_json(path)

    def test_no_special_disk_or_ssh_files(self):
        (self.root / ".local/vm").mkdir(parents=True)
        os.mkfifo(self.root / ".local/vm/root.qcow2")
        self.reject(self.data)

    def test_duplicate_resolved_paths_rejected(self):
        self.reject({**self.data, "nvram_file": self.data["disk_image"]})

    def test_external_provider_and_enrollment_identifiers(self):
        config = VMConfig.from_data(self.root, {**self.data, "provider": "external", "ssh_host": "vm.example.org"})
        self.assertEqual(config.values["provider"], "external")
        for update in ({"ssh_host": "192.0.2.1"}, {"ssh_host": "$(touch pwn)"},
                       {"guest_uuid": "bad"}, {"guest_role": "production"},
                       {"guest_source_root": "/home/root/releases"}, {"ssh_user": "-oProxyCommand=x"}):
            self.reject({**self.data, **update})


class DiscoveryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.config = VMConfig.from_data(self.root, EXAMPLE)

    def test_bound_loopback_port_is_occupied_not_verified(self):
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            listener.listen()
            result = tcp_probe("127.0.0.1", listener.getsockname()[1])
        self.assertTrue(result["reachable"])
        self.assertFalse(result["identity_verified"])

    def test_refusal_and_timeout_are_distinct(self):
        for error, expected in ((ConnectionRefusedError(), False), (TimeoutError(), None)):
            with patch("aios_dev.doctor.socket.create_connection", side_effect=error):
                self.assertIs(tcp_probe("127.0.0.1", 2222)["reachable"], expected)

    def test_subprocess_is_an_array_without_shell_interpolation(self):
        executable = "/fixture/with spaces/ssh;$(false)"
        completed = subprocess.CompletedProcess([], 0, "", "OpenSSH fixture\n")
        with patch("aios_dev.doctor.shutil.which", return_value=executable), patch("aios_dev.doctor.subprocess.run", return_value=completed) as run:
            result = tool_info("ssh")
        self.assertTrue(result["available"])
        self.assertEqual(run.call_args.args[0], [executable, "-V"])
        self.assertFalse(run.call_args.kwargs.get("shell", False))

    def test_missing_and_timed_out_tools(self):
        with patch("aios_dev.doctor.shutil.which", return_value=None):
            self.assertFalse(tool_info("ssh")["available"])
        with patch("aios_dev.doctor.shutil.which", return_value="/ssh"), patch("aios_dev.doctor.subprocess.run", side_effect=subprocess.TimeoutExpired(["/ssh"], 3)):
            self.assertEqual(tool_info("ssh")["error"], "TimeoutExpired")

    def test_firmware_must_be_a_matching_pair(self):
        (self.root / "OVMF_CODE.4m.fd").touch()
        (self.root / "OVMF_VARS.fd").touch()
        self.assertFalse(find_firmware([self.root])["available"])
        (self.root / "OVMF_VARS.4m.fd").touch()
        self.assertTrue(find_firmware([self.root])["available"])

    def test_memory_units(self):
        path = self.root / "meminfo"
        path.write_text("MemTotal: 100 kB\nMemAvailable: 40 kB\nBogus: 999 MB\n")
        self.assertEqual(memory_info(path), {"total_bytes": 102400, "available_bytes": 40960})

    def process(self, pid, args):
        entry = self.root / str(pid)
        entry.mkdir()
        (entry / "comm").write_text("qemu-system-x86\n")
        (entry / "cmdline").write_bytes(b"\x00".join(a.encode() for a in args))
        (entry / "stat").write_text(f"{pid} (qemu-system-x86) S " + "0 " * 18 + "12345 0")

    def test_qemu_identity_is_exact_and_never_trusted_by_discovery(self):
        disk = str(self.config.paths["disk_image"])
        qmp = f"unix:{self.config.paths['qmp_socket']},server=on,wait=off"
        self.process(100, ["qemu", "-drive", f"file={disk}.other,if=virtio"])
        self.process(101, ["qemu", "-qmp", qmp, "-drive", f"file={disk},if=virtio", "-netdev", "user,id=n,hostfwd=tcp:127.0.0.1:2222-:22"])
        result = qemu_processes(self.config, self.root)
        self.assertEqual([p["pid"] for p in result["matches"]], [101])
        process = result["matches"][0]
        self.assertEqual(process["start_ticks"], 12345)
        self.assertTrue(process["disk_matches"] and process["qmp_matches"] and process["forward_matches"])
        self.assertFalse(process["identity_verified"])

    def test_ssh_permissions_are_observed_not_changed(self):
        path = self.config.paths["identity_file"]
        path.parent.mkdir(parents=True, mode=0o700)
        path.touch(mode=0o644)
        self.assertTrue(ssh_file_permissions(self.config))
        self.assertEqual(path.stat().st_mode & 0o777, 0o644)
        path.chmod(0o600)
        self.assertFalse(ssh_file_permissions(self.config))

    def test_missing_prerequisites_include_affected_operations(self):
        with patch("aios_dev.doctor.tool_info", return_value={"available": False}), patch("aios_dev.doctor.find_firmware", return_value={"available": False}), patch("aios_dev.doctor.tcp_probe", return_value={"reachable": True, "identity_verified": False}), patch("aios_dev.doctor.qemu_processes", return_value={"matches": [], "complete": True}):
            result = host_report(self.config)
        missing = result["missing_prerequisites"]
        self.assertTrue(any(p["prerequisite"] == "QEMU x86-64" for p in missing))
        self.assertTrue(all(p["affected_operations"] for p in missing))
        self.assertFalse(result["ssh_port"]["available_for_forwarding"])
        self.assertFalse(result["guest"]["identity_verified"])

    def test_partial_qemu_match_does_not_hide_port_conflict(self):
        process = {"qmp_matches": True, "disk_matches": False, "forward_matches": True, "same_user": True}
        with patch("aios_dev.doctor.tcp_probe", return_value={"reachable": True, "identity_verified": False}), patch("aios_dev.doctor.qemu_processes", return_value={"matches": [process], "complete": True}):
            result = host_report(self.config)
        self.assertTrue(any("port" in p["prerequisite"] for p in result["missing_prerequisites"]))

    def test_external_provider_does_not_require_local_hypervisor(self):
        config = VMConfig.from_data(self.root, {**EXAMPLE, "provider": "external", "ssh_host": "vm.example.org"})
        with patch("aios_dev.doctor.tool_info", side_effect=lambda name: {"available": name in ("python3", "ssh", "sftp")}), patch("aios_dev.doctor.find_firmware", return_value={"available": False}), patch("aios_dev.doctor.kvm_info", return_value={"exists": False, "read_write_access": False}), patch("aios_dev.doctor.tcp_probe") as probe:
            result = host_report(config)
        self.assertEqual(result["missing_prerequisites"], [])
        probe.assert_not_called()


class CommandTests(unittest.TestCase):
    def invoke(self, args):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            status = main(args)
        result = json.loads(output.getvalue())
        self.assertEqual(result["exit_status"], status)
        return status, result

    def test_every_pending_command_fails_without_execution(self):
        commands = ["doctor --guest", "vm start", "enroll", "sync", "build --target packages", "build --target system", "test --suite unit", "test --suite integration", "test --suite desktop", "benchmark --profile normal", "deploy --mode test", "deploy --mode commit", "logs --unit aios-sessiond --user tester", "artifacts pull", "vm snapshot --name known-good", "vm restore --name known-good"]
        with patch("aios_dev.doctor.subprocess.run") as run:
            for command in commands:
                with self.subTest(command=command):
                    status, result = self.invoke([*command.split(), "--json"])
                    self.assertEqual(status, 9)
                    self.assertEqual(result["error"]["code"], "UNSUPPORTED_CAPABILITY")
                    self.assertIsNone(result["release_digest"])
                    self.assertIsNone(result["artifact_path"])
            run.assert_not_called()

    def test_invalid_input_is_json_with_exit_two(self):
        for args in (["--json"], ["doctor", "--json"], ["--json", "build", "--target", "root-shell"]):
            status, result = self.invoke(args)
            self.assertEqual(status, 2)
            self.assertEqual(result["error"]["code"], "INVALID_ARGUMENT")

    def test_doctor_prerequisite_exit_and_host_target(self):
        with patch("aios_dev.cli.host_report", return_value={"missing_prerequisites": [{"prerequisite": "fixture"}]}):
            status, result = self.invoke(["doctor", "--host", "--json"])
        self.assertEqual(status, 3)
        self.assertEqual(result["target"], "host")

    @unittest.skipUnless(shutil.which("fish"), "fish is optional in the guest package test environment")
    def test_fish_invocation(self):
        result = subprocess.run([shutil.which("fish"), "--no-config", "-c", "python3 tools/devctl.py build --target packages --json"], cwd=ROOT, capture_output=True, text=True, check=False, timeout=5)
        self.assertEqual(result.returncode, 9, result.stderr)
        self.assertEqual(json.loads(result.stdout)["error"]["code"], "UNSUPPORTED_CAPABILITY")


if __name__ == "__main__":
    unittest.main()
