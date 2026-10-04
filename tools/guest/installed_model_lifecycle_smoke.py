#!/usr/bin/env python3
"""Read-only normal-user observation of fixed initial model lifecycle evidence."""
import json
import os
from pathlib import Path
import stat
import subprocess

import snapshot

REPORT = Path('/run/aios-model-acceptance/result.json')


def observe():
    identity = snapshot.identity()
    if Path('/etc/aios/model-test-profile').read_text().strip() != 'installed-normal-cpu-model-v1':
        raise RuntimeError('not the registered installed model image')
    result = subprocess.run(['/run/current-system/sw/bin/systemctl', 'show', 'aios-model-acceptance.service',
                             '--property=ActiveState', '--property=SubState', '--property=Result', '--property=ExecMainStatus'],
                            capture_output=True, text=True, check=True, timeout=10)
    unit = dict(line.split('=', 1) for line in result.stdout.splitlines() if '=' in line)
    if unit['ActiveState'] in ('activating', 'active') or not REPORT.exists() and unit['Result'] == 'success':
        return 3, {'schema_version': 1, 'state': 'pending', 'identity': identity, 'unit': unit}
    directory = REPORT.parent.lstat()
    if not stat.S_ISDIR(directory.st_mode) or directory.st_uid != 0 or directory.st_mode & 0o022:
        raise RuntimeError('unsafe model lifecycle evidence directory')
    descriptor = os.open(REPORT, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, 'rb') as file:
        before = os.fstat(file.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_uid != 0 or before.st_mode & 0o222 != 0o200 or before.st_size > 131072:
            raise RuntimeError('unsafe model lifecycle evidence file')
        report = json.loads(file.read(131073))
        after = os.fstat(file.fileno())
        if (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns):
            raise RuntimeError('model lifecycle evidence changed')
    if report['identity'] != identity or snapshot.identity() != identity:
        raise RuntimeError('model lifecycle evidence belongs to another boot or target')
    success = (report['verified'] is True and len(report['restarts']) == 4 and report['active_request_transport_lost'] is True
               and unit == {'ActiveState': 'inactive', 'SubState': 'dead', 'Result': 'success', 'ExecMainStatus': '0'})
    return (0 if success else 8), {'schema_version': 1, 'state': 'verified' if success else 'failed', 'unit': unit, 'proof': report}


if __name__ == '__main__':
    code, value = observe()
    print(json.dumps(value, sort_keys=True), flush=True)
    raise SystemExit(code)
