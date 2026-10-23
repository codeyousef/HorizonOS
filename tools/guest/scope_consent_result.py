"""Validate fixed installed native-consent evidence from the optional test."""
import json
from pathlib import Path
import re
import sys
import uuid


CASES = ('existing_xdg_only', 'preconsent_denied', 'forged_confirmation_denied',
         'native_cancel_blocks_enrollment', 'pending_root_change_denied',
         'native_enrollment_content_verified', 'secret_paths_and_renamed_key_denied',
         'ungranted_mutation_denied', 'foreign_client_denied', 'changed_source_denied',
         'revocation_blocked', 'stored_material_counts_verified', 'native_ui_consent_qualified',
         'broker_security_verified', 'unmanaged_startup_refused',
         'managed_extraction_provenance', 'broker_pdf_unicode_ranges', 'extraction_foreign_changed_revoked_denied', 'broker_survives_malformed_parser', 'unmanaged_worker_gets_no_descriptor', 'revocation_during_extraction_denied')


def validate(result):
    if type(result.get('schema_version')) is not int or result['schema_version'] != 1 or result.get('evidence_kind') != 'installed-public-files-native-consent-synthetic-desktop':
        raise ValueError('wrong native-consent report')
    if result.get('uid') != 1000 or any(result.get(case) is not True for case in CASES):
        raise ValueError('missing required native-consent case')
    identity = result['identity']
    expected = {'dmi_uuid':'f905c5bf-7f82-478b-96fa-4f8827d8e254',
                'installation_uuid':'ec31bea9-a87a-4f8d-bd97-975eb9d69f8f',
                'guest_role':'development', 'disk_serial':'root',
                'management_channel':'nixos-test-driver'}
    if any(identity.get(key) != value for key, value in expected.items()):
        raise ValueError('wrong consent fixture target')
    try:
        if str(uuid.UUID(identity['boot_id'])) != identity['boot_id']:
            raise ValueError('noncanonical boot identity')
    except (KeyError, TypeError, AttributeError):
        raise ValueError('missing boot identity') from None
    if not re.fullmatch(r'[0-9a-f]{32}', identity.get('machine_id', '')):
        raise ValueError('missing machine identity')
    if not re.fullmatch(r'/nix/store/[0-9a-z]{32}-nixos-system-[^/]+', identity.get('current_system', '')):
        raise ValueError('missing installed system identity')
    if not re.fullmatch(r'SHA256:[A-Za-z0-9+/]{43}', identity.get('host_key_fingerprint', '')):
        raise ValueError('missing trusted key pin')
    if not result['installed_executable'].startswith('/nix/store/') or not result['installed_executable'].endswith('/bin/aios-sessiond'):
        raise ValueError('wrong installed broker')
    security = result.get('broker_security', {})
    if security.get('NoNewPrivs') != '1' or security.get('Seccomp') != '2' or security.get('Uid', '').split() != ['1000']*4:
        raise ValueError('installed broker protections missing')
    if any(security.get(key) != '0000000000000000' for key in ('CapEff','CapPrm','CapInh','CapAmb')):
        raise ValueError('installed broker has capabilities')
    if result.get('production_image_qualified') is not False or result.get('ranked_index_qualified') is not False:
        raise ValueError('fixture overclaims production or ranked index')
    purge = result['persistent_purge']
    if purge['access_blocked'] is not True or purge['purge_pending'] is not False or purge['cached_records_purged'] != 3:
        raise ValueError('native revoke did not finish purge')
    views = result['native_ui_views']
    if len(views) != 3 or any(view['exact_root_visible'] is not True or view['content_only_visible'] is not True for view in views):
        raise ValueError('native UI evidence missing')
    if any(type(view.get('native_pid')) is not int or view['native_pid'] <= 0 for view in views):
        raise ValueError('missing actual native process')
    if [view['mode'] for view in views] != ['inspect', 'inspect', 'allow']:
        raise ValueError('actual native button action not recorded')
    return result


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise ValueError('fixed job directory required')
    target = (Path(sys.argv[1]) / 'scope-fixture-result' / 'scope-consent.json').resolve(strict=True)
    if not target.is_relative_to('/nix/store') or target.stat().st_size > 65536:
        raise ValueError('invalid native report path or size')
    print('SCOPE_CONSENT_RESULT ' + json.dumps(validate(json.loads(target.read_text())), sort_keys=True))
