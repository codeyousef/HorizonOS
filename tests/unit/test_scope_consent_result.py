"""Negative evidence gates, not native consent acceptance."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('scope_consent_result', Path(__file__).resolve().parents[2] / 'tools/guest/scope_consent_result.py')
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)


class ConsentResultTests(unittest.TestCase):
    def setUp(self):
        self.result = dict(schema_version=1, uid=1000,
            evidence_kind='installed-public-files-native-consent-synthetic-desktop',
            identity=dict(dmi_uuid='f905c5bf-7f82-478b-96fa-4f8827d8e254',
                installation_uuid='ec31bea9-a87a-4f8d-bd97-975eb9d69f8f',
                guest_role='development', disk_serial='root', management_channel='nixos-test-driver',
                host_key_fingerprint='SHA256:'+'a'*43,
                boot_id='c08ae052-b41c-481a-9865-d289aa611802', machine_id='a'*32,
                current_system='/nix/store/'+'a'*32+'-nixos-system-machine-test'),
            installed_executable='/nix/store/fixture-aios-core/bin/aios-sessiond',
            broker_security=dict(NoNewPrivs='1', Seccomp='2', Uid='1000 1000 1000 1000',
                CapEff='0000000000000000',CapPrm='0000000000000000',CapInh='0000000000000000',CapAmb='0000000000000000'),
            production_image_qualified=False, ranked_index_qualified=False,
            persistent_purge=dict(access_blocked=True, purge_pending=False, cached_records_purged=3),
            native_ui_views=[dict(mode=mode, native_pid=1234, exact_root_visible=True, content_only_visible=True)
                             for mode in ('inspect','inspect','allow')])
        self.result.update({case: True for case in validator.CASES})

    def test_required_native_cases_cannot_be_skipped_or_missing(self):
        self.assertEqual(validator.validate(self.result), self.result)
        for case in validator.CASES:
            for value in (False, None, 'skipped'):
                with self.subTest(case=case, value=value), self.assertRaises(ValueError):
                    validator.validate({**self.result, case:value})
            changed = self.result.copy()
            del changed[case]
            with self.assertRaises(ValueError):
                validator.validate(changed)

    def test_changed_identity_pending_purge_and_overclaims_refuse(self):
        for field, value in [('schema_version', True), ('uid', 1001),
                ('installed_executable','/tmp/aios-sessiond'), ('production_image_qualified',True),
                ('ranked_index_qualified',True)]:
            with self.assertRaises(ValueError):
                validator.validate({**self.result, field:value})
        for section, field, value in [('identity','dmi_uuid','other'),
                ('identity','installation_uuid','other'), ('identity','guest_role','host'),
                ('persistent_purge','purge_pending',True), ('persistent_purge','cached_records_purged',0)]:
            changed=copy.deepcopy(self.result); changed[section][field]=value
            with self.assertRaises(ValueError):
                validator.validate(changed)
        for mode in ('inspect', 'forged'):
            changed=copy.deepcopy(self.result); changed['native_ui_views'][2]['mode']=mode
            with self.assertRaises(ValueError):
                validator.validate(changed)

    def test_missing_runtime_identity_and_native_process_refuse(self):
        for field in ('boot_id','machine_id','current_system','host_key_fingerprint'):
            changed=copy.deepcopy(self.result); del changed['identity'][field]
            with self.subTest(field=field), self.assertRaises(ValueError):
                validator.validate(changed)
        for value in (None, True, 0, -1):
            changed=copy.deepcopy(self.result); changed['native_ui_views'][0]['native_pid']=value
            with self.subTest(pid=value), self.assertRaises(ValueError):
                validator.validate(changed)

    def test_missing_or_changed_broker_protections_refuse(self):
        for field, value in [('NoNewPrivs','0'), ('Seccomp','0'), ('Uid','0 0 0 0'),
                ('CapEff','0000000000000001'), ('CapPrm','0000000000000001'),
                ('CapInh','0000000000000001'), ('CapAmb','0000000000000001')]:
            changed=copy.deepcopy(self.result);changed['broker_security'][field]=value
            with self.subTest(field=field), self.assertRaises(ValueError):
                validator.validate(changed)
        changed=copy.deepcopy(self.result); del changed['broker_security']
        with self.assertRaises(ValueError):
            validator.validate(changed)
