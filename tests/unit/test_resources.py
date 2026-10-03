"""Measured host resource fixtures; no inference capability is claimed."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from aios_dev.config import VMConfig, load_config
from aios_dev.errors import DevctlError
from aios_dev.provision import prepare_plan, select_fresh_defaults
from aios_dev.resources import GIB, recommend


def measured(cpu=32, memory=40, disk=200):
    return {"logical_cpus":cpu, "memory":{"available_bytes":memory * GIB},
            "disk":{"free_bytes":disk * GIB}}


class ResourceTests(unittest.TestCase):
    def test_defaults_and_reduction_preserve_host_reserves(self):
        full = recommend(measured())
        self.assertEqual((full["vcpus"], full["memory_mib"], full["disk_gib"]), (8,16384,96))
        smaller = recommend(measured(4,8,64))
        self.assertEqual((smaller["vcpus"], smaller["memory_mib"], smaller["disk_gib"]), (2,6144,56))
        self.assertLessEqual(smaller["memory_mib"] * 1024**2 + smaller["host_memory_reserve_bytes"],8 * GIB)
        self.assertLessEqual(smaller["disk_gib"] * GIB + smaller["host_disk_reserve_bytes"],64 * GIB)

    def test_unknown_or_insufficient_resources_never_guess(self):
        for report in ({}, measured(1,4,200), measured(4,8,50), measured(True,8,200)):
            with self.subTest(report=report), self.assertRaises(DevctlError):
                recommend(report)

    def test_selection_is_frozen_before_uuid_and_existing_plans_stay_exact(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "dev").mkdir()
            example = json.loads((ROOT / "dev/vm.example.json").read_text())
            (root / "dev/vm.example.json").write_text(json.dumps(example))
            original = load_config(root)
            with patch("aios_dev.provision.host_report", return_value=measured(4,8,64)):
                selected = select_fresh_defaults(original)
            plan = prepare_plan(selected)
            with patch("aios_dev.provision.host_report") as discovery:
                again = select_fresh_defaults(load_config(root))
                self.assertEqual(prepare_plan(again),plan)
                self.assertIs(select_fresh_defaults(original),original)
                discovery.assert_not_called()
            self.assertEqual(again.values["memory_mib"],6144)
            self.assertFalse(selected.paths["disk_image"].exists())
            self.assertEqual(json.loads((root / ".local/provisioning-resources.json").read_text())["configuration"],selected.values)

    def test_explicit_configuration_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            config = VMConfig.from_data(Path(directory),json.loads((ROOT / "dev/vm.example.json").read_text()),configured=True)
            with patch("aios_dev.provision.host_report") as discovery:
                self.assertIs(select_fresh_defaults(config),config)
                discovery.assert_not_called()


if __name__ == "__main__":
    unittest.main()
