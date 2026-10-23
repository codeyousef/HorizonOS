"""Strict source-bound installed extractor fixture report validation."""
import json
import re
from pathlib import Path
import sys
import uuid

CASES = ('text_unicode_locators','source_hash_verified','output_bound','source_bound_and_range',
         'metadata_only_formats','writable_fd_refused','pdf_physical_pages','pdf_range',
         'malformed_isolated','encrypted_no_content','scanned_no_interpretation',
         'page_bound_and_range','worker_restart_after_failure','resource_profile_probes_qualified',
         'caller_survived_resource_failure','large_pdf_range','raw_key_range_denied','mixed_pdf_completeness','metadata_cannot_spoof_pages','archive_bomb_not_expanded')


def validate(result):
    if type(result.get('schema_version')) is not int or result['schema_version'] != 1:
        raise ValueError('wrong extraction evidence schema')
    if result.get('evidence_kind') != 'installed-descriptor-extractor-synthetic-corpus':
        raise ValueError('wrong extraction evidence kind')
    if any(result.get(case) is not True for case in CASES):
        raise ValueError('missing extraction acceptance case')
    if result.get('production_index_integrated') is not False:
        raise ValueError('fixture overclaims index integration')
    i = result['identity']
    expected = {'dmi_uuid':'3a491b68-9b63-4d24-b232-8d7ce1ba044c',
                'installation_uuid':'699932ee-271f-429c-b478-edb05b33c5d4',
                'disk_serial':'root','guest_role':'development','management_channel':'nixos-test-driver'}
    if any(i.get(k) != v for k,v in expected.items()):
        raise ValueError('wrong extractor fixture identity')
    if str(uuid.UUID(i.get('boot_id',''))) != i.get('boot_id') or not re.fullmatch('[0-9a-f]{32}', i.get('machine_id','')):
        raise ValueError('missing live boot identity')
    if not re.fullmatch(r'SHA256:[A-Za-z0-9+/]{43}', i.get('host_key_fingerprint','')) or not re.fullmatch(r'/nix/store/[0-9a-z]{32}-nixos-system-[^/]+', i.get('current_system','')):
        raise ValueError('missing installed system/trust binding')
    s = result['worker_security']
    if any(type(s.get(k)) is not int or s[k] != value for k,value in [('uid',1000),('nnp',1),('seccomp',2)]) or s.get('capabilities_empty') is not True or s.get('private_namespaces') is not True:
        raise ValueError('worker isolation not observed')
    if type(s.get('pid')) is not int or s['pid'] <= 1 or not s.get('executable','').startswith('/nix/store/') or not s['executable'].endswith('/bin/aios-extract'):
        raise ValueError('actual installed worker missing')
    for name, expected in [('memory','oom-kill'),('cpu','timeout')]:
        p = result['resource_probes'][name]
        if p.get('Result') != expected or p.get('MemoryMax') != '536870912' or p.get('MemorySwapMax') != '0' or p.get('RuntimeMaxUSec') != '10s' or p.get('CPUQuotaPerSecUSec') != '900ms' or p.get('TasksMax') != '8' or p.get('KillMode') != 'control-group':
            raise ValueError('kernel resource limit not qualified')
        if name == 'cpu' and not 0 < int(p['CPUUsageNSec']) < 10_000_000_000:
            raise ValueError('aggregate CPU budget exceeded')
    return result


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise ValueError('fixed job directory required')
    p = (Path(sys.argv[1]) / 'scope-fixture-result' / 'extract.json').resolve(strict=True)
    if not p.is_relative_to('/nix/store') or p.stat().st_size > 65536:
        raise ValueError('invalid extractor report path')
    print('EXTRACT_RESULT ' + json.dumps(validate(json.loads(p.read_text())), sort_keys=True))
