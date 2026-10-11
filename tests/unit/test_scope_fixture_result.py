"""Evidence refusal checks; these do not qualify native OS behavior."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('scope_fixture_result', Path(__file__).resolve().parents[2] / 'tools/guest/scope_fixture_result.py')
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)


class ScopeResultTests(unittest.TestCase):
    def setUp(self):
        self.result = dict(schema_version=1, uid=1000,
            evidence_kind='native-synthetic-UID-mount-cache-component-test',
            identity=dict(dmi_uuid='e64a1bf0-814b-4dc0-94a3-04c539f41cd2',
                installation_uuid='0d698869-1d37-4776-b3c7-30c7689a2d15',
                guest_role='development', disk_serial='root',
                management_channel='nixos-test-driver', host_key_fingerprint='SHA256:fixture'),
            native_ui_consent_qualified=False, production_installation_qualified=False,
            persistent_purge=dict(access_blocked=True, purge_pending=False, cached_records_purged=3))
        for key in ('preconsent_denied', 'foreign_owned_readable_file_denied', 'symlink_escape_denied',
                    'new_mount_denied', 'retained_handle_mount_change_denied',
                    'replaced_root_blocks_cached_snippet', 'actual_foreign_uid_cache_denied'):
            self.result[key] = True

    def test_required_case_missing_skipped_or_failed_refuses(self):
        self.assertEqual(validator.validate(self.result), self.result)
        for key in ('preconsent_denied', 'foreign_owned_readable_file_denied', 'symlink_escape_denied',
                    'new_mount_denied', 'retained_handle_mount_change_denied',
                    'replaced_root_blocks_cached_snippet', 'actual_foreign_uid_cache_denied'):
            for value in (False, None, 'skipped'):
                changed = {**self.result, key: value}
                with self.subTest(key=key, value=value), self.assertRaises(AssertionError):
                    validator.validate(changed)
            changed = self.result.copy()
            del changed[key]
            with self.assertRaises(KeyError):
                validator.validate(changed)

    def test_wrong_target_pending_purge_and_overclaim_refuse(self):
        for field, value in [('uid', 1001), ('native_ui_consent_qualified', True),
                             ('production_installation_qualified', True)]:
            with self.assertRaises(AssertionError):
                validator.validate({**self.result, field: value})
        for section, field, value in [('identity', 'dmi_uuid', 'other'),
                ('identity', 'installation_uuid', 'other'), ('identity', 'guest_role', 'host'),
                ('identity', 'disk_serial', 'AIOS_DEV_ROOT'),
                ('persistent_purge', 'purge_pending', True),
                ('persistent_purge', 'access_blocked', False),
                ('persistent_purge', 'cached_records_purged', 0)]:
            changed = copy.deepcopy(self.result)
            changed[section][field] = value
            with self.assertRaises(AssertionError):
                validator.validate(changed)
