"""Fixture-only refusal tests for the installed graph report acceptance gate."""
import copy
import unittest
import installed_graph_owner_smoke as gate
from graph_owner_preflight import raw_timer_trigger, denied_probe_valid, event_proof_valid, event_status_ready, device_proof_valid, metadata_proof_valid

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

    def event_fixture(self):
        status = {'boot': 'fixture-boot', 'event_watcher_installed': True, 'event_watcher_error': None,
                  'model_invoked': False, 'execution_authority': False, 'systemd_notifications': 0,
                  'event_reconciliations': 0, 'last_attempt': {'boot': 'fixture-boot', 'monotonic_ns': 10}}
        after = dict(status, systemd_notifications=3, event_reconciliations=1)
        return {'before': status, 'after': after, 'service': {'properties': {
            'captured': {'boot': 'fixture-boot', 'monotonic_ns': 20},
            'observation': {'load_state': 'loaded', 'active_state': 'active', 'sub_state': 'running'}}},
            'native': {'LoadState': 'loaded', 'ActiveState': 'active', 'SubState': 'running'}}

    def test_event_gate_requires_real_notifications_and_new_committed_observation(self):
        value = self.event_fixture()
        self.assertTrue(event_proof_valid(value, 'fixture-boot'))
        for field, other in [('event_watcher_installed', False), ('event_watcher_error', 'disconnected'),
                             ('systemd_notifications', 0), ('systemd_notifications', True),
                             ('event_reconciliations', 0), ('event_reconciliations', True),
                             ('boot', 'foreign-boot'), ('model_invoked', True)]:
            altered = copy.deepcopy(value); altered['after'][field] = other
            self.assertFalse(event_proof_valid(altered, 'fixture-boot'))
        for field, other in [('boot', 'foreign-boot'), ('monotonic_ns', 9), ('monotonic_ns', True)]:
            altered = copy.deepcopy(value); altered['service']['properties']['captured'][field] = other
            self.assertFalse(event_proof_valid(altered, 'fixture-boot'))
        altered = copy.deepcopy(value); altered['native']['ActiveState'] = 'failed'
        self.assertFalse(event_proof_valid(altered, 'fixture-boot'))

    def test_event_baseline_requires_recovery_and_fresh_post_baseline_counters(self):
        value = self.event_fixture()
        unavailable = dict(value['before'], event_watcher_installed=False,
                           event_watcher_error='BOUNDED_DRAIN_LOSS', systemd_notifications=63)
        self.assertFalse(event_status_ready(unavailable, 'fixture-boot'))
        recovered = dict(value['before'], systemd_notifications=130, event_reconciliations=9)
        self.assertTrue(event_status_ready(recovered, 'fixture-boot'))
        for field, other in [('boot', 'other'), ('event_watcher_installed', 1),
                             ('event_watcher_error', 'disconnected'), ('systemd_notifications', True),
                             ('event_reconciliations', -1), ('model_invoked', True),
                             ('execution_authority', True)]:
            self.assertFalse(event_status_ready(dict(recovered, **{field: other}), 'fixture-boot'))
        self.assertFalse(event_status_ready({}, 'fixture-boot'))
        value['before'] = recovered
        value['after'] = dict(recovered)
        # Recovery itself and old boot events cannot satisfy the new probe.
        self.assertFalse(event_proof_valid(value, 'fixture-boot'))
        value['after'] = dict(recovered, systemd_notifications=131, event_reconciliations=10)
        self.assertTrue(event_proof_valid(value, 'fixture-boot'))
        value['before'] = unavailable
        self.assertFalse(event_proof_valid(value, 'fixture-boot'))

    def device_fixture(self):
        return {'execution_authority': False, 'native_syspath': '/sys/devices/fixture', 'native_devnum': [252, 0],
            'native_properties': {'ID_SERIAL': 'fixture-native-serial'},
            'selected': {'source_truth': 'running', 'properties': {'execution_authority': False,
                'live_identity_retained': False, 'captured': {'boot': 'fixture-boot'},
                'device': {'syspath': '/sys/devices/fixture', 'major': 252, 'minor': 0,
                    'serial': 'fixture-native-serial', 'serial_short': None, 'wwn': None, 'bus': None, 'model': None, 'vendor': None}}}}

    def metadata_fixture(self):
        manifest = {'fixture': 'typed built data'}
        return {'execution_authority': False, 'native': {'manifest': manifest, 'manifest_sha256': 'a'*64, 'catalog_sha256': 'b'*64},
            'metadata': {'source_truth': 'built', 'freshness': 'Current', 'captured': {'boot': 'fixture-boot'},
                'approved_manifest_verified': False, 'managed_transaction': None, 'runtime_postconditions_verified': False,
                'complete_package_inventory_verified': False, 'configuration': {'running_closure': 'fixture-system',
                    'manifest_sha256': 'a'*64, 'catalog_sha256': 'b'*64, 'configuration': manifest}}}

    def test_built_metadata_gate_refuses_wrong_hash_scope_or_invented_approval(self):
        identity = {'boot_id': 'fixture-boot', 'current_system': 'fixture-system'}
        value = self.metadata_fixture(); self.assertTrue(metadata_proof_valid(value, identity))
        for field, other in [('source_truth', 'intended'), ('freshness', 'Stale'), ('freshness', 'Unknown'),
                             ('approved_manifest_verified', True), ('managed_transaction', 'guessed'),
                             ('runtime_postconditions_verified', True), ('complete_package_inventory_verified', True)]:
            altered = copy.deepcopy(value); altered['metadata'][field] = other
            self.assertFalse(metadata_proof_valid(altered, identity))
        for field in ('manifest_sha256', 'catalog_sha256', 'running_closure', 'configuration'):
            altered = copy.deepcopy(value); altered['metadata']['configuration'][field] = 'foreign'
            self.assertFalse(metadata_proof_valid(altered, identity))
        altered = copy.deepcopy(value); altered['metadata']['captured']['boot'] = 'foreign'
        self.assertFalse(metadata_proof_valid(altered, identity))
        altered = copy.deepcopy(value); altered['execution_authority'] = True
        self.assertFalse(metadata_proof_valid(altered, identity))
        for other in (True, 'short', 'A'*64):
            altered = copy.deepcopy(value); altered['native']['manifest_sha256'] = other
            altered['metadata']['configuration']['manifest_sha256'] = other
            self.assertFalse(metadata_proof_valid(altered, identity))

    def test_device_gate_refuses_foreign_kernel_identity_or_invented_missing_serial(self):
        identity = {'boot_id': 'fixture-boot', 'disk_serial': 'fixture-native-serial'}
        value = self.device_fixture(); self.assertTrue(device_proof_valid(value, identity))
        for field, other in [('syspath', '/sys/devices/other'), ('major', True), ('minor', 1),
                             ('serial', 'invented'), ('serial_short', 'invented'), ('wwn', 'invented')]:
            altered = copy.deepcopy(value); altered['selected']['properties']['device'][field] = other
            self.assertFalse(device_proof_valid(altered, identity))
        for field, other in [('execution_authority', True), ('live_identity_retained', True)]:
            altered = copy.deepcopy(value); altered['selected']['properties'][field] = other
            self.assertFalse(device_proof_valid(altered, identity))
        altered = copy.deepcopy(value); altered['selected']['properties']['captured']['boot'] = 'foreign-boot'
        self.assertFalse(device_proof_valid(altered, identity))

    def fixture(self):
        identity = {"boot_id": "fixture-boot", "current_system": "fixture-system", "disk_serial": "fixture-native-serial"}
        steps = {k: {} for k in gate.REQUIRED}
        steps.update({"service_comparison": {"unknown_properties_preserved": True},
            "systemd_events": self.event_fixture(),
            "startup_devices": self.device_fixture(),
            "startup_metadata": self.metadata_fixture(),
            "startup_generations": {"execution_authority": False, "data": {"freshness": "Current", "pointers": {"running_closure": "fixture-system", "bootloader_entry": None, "managed_transaction": None}}},
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

    def test_generation_gate_rejects_stale_foreign_or_inferred_provenance(self):
        identity, value = self.fixture()
        for field, other in (('freshness', 'Unknown'), ('freshness', 'Stale')):
            altered = copy.deepcopy(value); altered['steps']['startup_generations']['data'][field] = other
            self.assertFalse(gate.qualified(altered, identity))
        for field, other in (('running_closure', 'foreign-system'), ('bootloader_entry', 'guessed-entry'), ('managed_transaction', 'guessed-transaction')):
            altered = copy.deepcopy(value); altered['steps']['startup_generations']['data']['pointers'][field] = other
            self.assertFalse(gate.qualified(altered, identity))
        value['steps']['startup_generations']['execution_authority'] = True
        self.assertFalse(gate.qualified(value, identity))

if __name__ == "__main__":
    unittest.main()
