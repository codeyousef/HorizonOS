"""Pull one fixed component-fixture result into a tracked job's bounded log."""
import json
from pathlib import Path
import sys


def validate(result):
    assert result['schema_version'] == 1
    assert result['evidence_kind'] == 'native-synthetic-UID-mount-cache-component-test'
    assert result['uid'] == 1000
    identity = result['identity']
    assert identity['dmi_uuid'] == 'e64a1bf0-814b-4dc0-94a3-04c539f41cd2'
    assert identity['installation_uuid'] == '0d698869-1d37-4776-b3c7-30c7689a2d15'
    assert identity['guest_role'] == 'development' and identity['disk_serial'] == 'root'
    assert identity['management_channel'] == 'nixos-test-driver'
    assert identity['host_key_fingerprint'].startswith('SHA256:')
    assert result['native_ui_consent_qualified'] is False
    assert result['production_installation_qualified'] is False
    for key in ('preconsent_denied', 'foreign_owned_readable_file_denied', 'symlink_escape_denied',
                'new_mount_denied', 'retained_handle_mount_change_denied', 'replaced_root_blocks_cached_snippet',
                'actual_foreign_uid_cache_denied'):
        assert result[key] is True
    purge = result['persistent_purge']
    assert purge['access_blocked'] is True and purge['purge_pending'] is False
    assert purge['cached_records_purged'] == 3
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    target = (Path(sys.argv[1]) / 'scope-fixture-result' / 'scope-boundaries.json').resolve(strict=True)
    assert target.is_relative_to('/nix/store') and target.name == 'scope-boundaries.json'
    assert target.stat().st_size <= 65536
    result = validate(json.loads(target.read_text()))
    print('SCOPE_BOUNDARIES_RESULT ' + json.dumps(result, sort_keys=True))
