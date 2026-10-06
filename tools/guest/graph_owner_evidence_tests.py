"""Fixture-only refusal tests for the installed graph report acceptance gate."""
import copy
import unittest
import installed_graph_owner_smoke as gate
from graph_owner_preflight import raw_timer_trigger, denied_probe_valid

class GraphEvidenceTests(unittest.TestCase):
    def test_denial_requires_original_dev_process_exit(self):
        value = self.denial_unit()
        self.assertTrue(denied_probe_valid(value))
        for key, other in (('User', 'root'), ('Result', 'resources'), ('ExecMainCode', '2'),
                           ('ExecMainStatus', '203'), ('ExecMainStatus', '0'), ('ActiveState', 'active')):
            altered = dict(value); altered[key] = other
            with self.subTest(key=key, other=other): self.assertFalse(denied_probe_valid(altered))

    def denial_unit(self):
        return {'User': 'dev', 'Result': 'exit-code', 'ExecMainCode': '1', 'ExecMainStatus': '1',
                'ActiveState': 'failed', 'SubState': 'failed'}

    def test_native_timer_timestamp_refuses_display_text_or_wrong_wire_type(self):
        self.assertEqual(raw_timer_trigger(b't 909503463'), 909503463)
        for text in (b'15min 9.503463s', b's 909503463', b't -1', b't 0x1', b't 1 2', b't 18446744073709551616'):
            with self.subTest(text=text), self.assertRaises(RuntimeError):
                raw_timer_trigger(text)

    def fixture(self):
        identity = {"boot_id": "fixture-boot"}
        steps = {k: {} for k in gate.REQUIRED}
        steps.update({"service_comparison": {"unknown_properties_preserved": True},
            "foreign_uid_denied": {"upstream_exit": 1, "native_unit": self.denial_unit()},
            "corruption_recovery": {"ledger_unchanged": True},
            "real_timer": {"elapsed_ns": 900_000_000_000, "status": {"model_invoked": False, "execution_authority": False}},
            "final_process": {"zero_capabilities": True, "private_modes_verified": True}})
        for k in ("startup_desktop", "outage_desktop", "final_desktop"):
            steps[k] = {"boot_id": identity["boot_id"]}
        return identity, {"schema_version": 1, "identity": identity, "verified": True,
            "evidence_kind": "actual-installed-graph-owner", "steps": steps}

    def test_missing_required_gate_cannot_pass(self):
        identity, value = self.fixture()
        self.assertTrue(gate.qualified(value, identity))
        for k in gate.REQUIRED:
            altered = copy.deepcopy(value); del altered["steps"][k]
            with self.subTest(k=k): self.assertFalse(gate.qualified(altered, identity))

    def test_shortened_or_boolean_elapsed_time_cannot_pass(self):
        identity, value = self.fixture()
        for duration in (0, 899_999_999_999, True, "900000000000"):
            value["steps"]["real_timer"]["elapsed_ns"] = duration
            self.assertFalse(gate.qualified(value, identity))

    def test_foreign_boot_or_authority_cannot_pass(self):
        identity, value = self.fixture()
        self.assertFalse(gate.qualified(value, {"boot_id": "other-boot"}))
        for k in ("startup_desktop", "outage_desktop", "final_desktop"):
            altered = copy.deepcopy(value); altered["steps"][k]["boot_id"] = "other-boot"
            self.assertFalse(gate.qualified(altered, identity))
        for k in ("model_invoked", "execution_authority"):
            altered = copy.deepcopy(value); altered["steps"]["real_timer"]["status"][k] = True
            self.assertFalse(gate.qualified(altered, identity))

    def test_false_denial_or_schema_drift_cannot_pass(self):
        identity, value = self.fixture()
        for exit_code in (0, True, "1"):
            altered = copy.deepcopy(value); altered["steps"]["foreign_uid_denied"]["upstream_exit"] = exit_code
            self.assertFalse(gate.qualified(altered, identity))
        value["schema_version"] = True
        self.assertFalse(gate.qualified(value, identity))
        value["schema_version"] = 1; value["unexpected"] = "field"
        self.assertFalse(gate.qualified(value, identity))

if __name__ == "__main__":
    unittest.main()
