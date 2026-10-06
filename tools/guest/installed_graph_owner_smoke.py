#!/usr/bin/env python3
"""Normal-UID read-only observation of fixed installed graph qualification."""
import json
import os
from pathlib import Path
import stat
import subprocess

import snapshot
from graph_owner_preflight import denied_probe_valid, event_proof_valid, device_proof_valid, metadata_proof_valid

REPORT = Path('/run/aios-graph-acceptance/result.json')
PHASE = REPORT.with_name('phase.json')
REQUIRED = {'startup', 'startup_desktop', 'startup_generations', 'startup_devices', 'startup_metadata', 'systemd_events', 'service_comparison', 'foreign_uid_denied', 'restart', 'outage_desktop',
            'outage_ssh', 'corruption_recovery', 'real_timer', 'final_desktop', 'final_process'}


def read_proof(path):
    parent = path.parent.lstat()
    if not stat.S_ISDIR(parent.st_mode) or parent.st_uid != 0 or stat.S_IMODE(parent.st_mode) != 0o755:
        raise RuntimeError('unsafe graph qualification directory')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as file:
        before = os.fstat(file.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_uid != 0 or stat.S_IMODE(before.st_mode) != 0o444 or before.st_nlink != 1 or before.st_size > 131072:
            raise RuntimeError('unsafe graph qualification report')
        value = snapshot.decode(file.read(131073))
        after = os.fstat(file.fileno())
        if (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns):
            raise RuntimeError('graph qualification report changed')
        return value


def qualified(value, identity):
    try:
        steps = value['steps']
        return (set(value) == {'schema_version', 'identity', 'evidence_kind', 'verified', 'steps'}
                and type(value['schema_version']) is int and value['schema_version'] == 1 and value['identity'] == identity and value['verified'] is True
                and value['evidence_kind'] == 'actual-installed-graph-owner' and set(steps) == REQUIRED
                and steps['service_comparison']['unknown_properties_preserved'] is True
                and event_proof_valid(steps['systemd_events'], identity['boot_id'])
                and device_proof_valid(steps['startup_devices'], identity)
                and metadata_proof_valid(steps['startup_metadata'], identity)
                and steps['startup_generations']['execution_authority'] is False
                and steps['startup_generations']['data']['freshness'] == 'Current'
                and steps['startup_generations']['data']['pointers']['running_closure'] == identity['current_system']
                and steps['startup_generations']['data']['pointers']['bootloader_entry'] is None
                and steps['startup_generations']['data']['pointers']['managed_transaction'] is None
                and type(steps['foreign_uid_denied']['upstream_exit']) is int and steps['foreign_uid_denied']['upstream_exit'] != 0
                and denied_probe_valid(steps['foreign_uid_denied']['native_unit'])
                and steps['corruption_recovery']['ledger_unchanged'] is True
                and type(steps['real_timer']['elapsed_ns']) is int and steps['real_timer']['elapsed_ns'] >= 900_000_000_000
                and steps['real_timer']['status']['model_invoked'] is False
                and steps['real_timer']['status']['execution_authority'] is False
                and steps['final_process']['zero_capabilities'] is True
                and steps['final_process']['private_modes_verified'] is True
                and all(steps[k]['boot_id'] == identity['boot_id'] for k in ('startup_desktop', 'outage_desktop', 'final_desktop')))
    except (KeyError, TypeError):
        return False


def observe():
    identity = snapshot.identity()
    if Path('/etc/aios/graph-test-profile').read_text().strip() != 'fixed-installed-graph-owner-v1':
        raise RuntimeError('not the fixed installed graph image')
    result = subprocess.run(['/run/current-system/sw/bin/systemctl', 'show', 'aios-graph-acceptance.service',
                             '--property=ActiveState', '--property=SubState', '--property=Result', '--property=ExecMainStatus'],
                            capture_output=True, text=True, timeout=10, check=True)
    unit = dict(line.split('=', 1) for line in result.stdout.splitlines() if '=' in line)
    if unit['ActiveState'] in ('active', 'activating'):
        phase = read_proof(PHASE) if PHASE.exists() else None
        if phase is not None and (phase.get('identity') != identity or phase.get('phase') not in ('startup', 'graph_outage', 'waiting_real_timer', 'complete')):
            raise RuntimeError('graph phase target differs')
        value = {'schema_version': 1, 'state': 'pending', 'identity': identity, 'unit': unit, 'phase': phase}
        code = 3
    else:
        proof = read_proof(REPORT)
        if proof.get('identity') != identity:
            raise RuntimeError('graph report target differs')
        success = qualified(proof, identity) and unit == {'ActiveState': 'inactive', 'SubState': 'dead', 'Result': 'success', 'ExecMainStatus': '0'}
        value = {'schema_version': 1, 'state': 'verified' if success else 'failed', 'identity': identity, 'unit': unit, 'proof': proof}
        code = 0 if success else 8
    if snapshot.identity() != identity:
        raise RuntimeError('graph report identity changed')
    return code, value


if __name__ == '__main__':
    code, value = observe()
    print(json.dumps(value, sort_keys=True), flush=True)
    raise SystemExit(code)
