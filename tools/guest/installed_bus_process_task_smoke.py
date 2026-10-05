#!/usr/bin/env python3
"""Persistent native bus client against original installed broker/CPU model."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

import snapshot
from model_public_fixture import Broker
from installed_process_task_smoke import require_answer


def main():
    identity = snapshot.identity()
    if os.getuid() != 1000 or os.geteuid() != 1000 or identity['guest_role'] != 'development':
        raise RuntimeError('requires enrolled normal development subject')
    broker = Broker(uid=1000)
    try:
        release = Path(__file__).resolve().parents[2]
        unit = subprocess.check_output(['/run/current-system/sw/bin/systemctl', '--system', 'show',
                                       'aios-model.service', '--property=ExecStart'], timeout=5).decode()
        match = re.search(r'--model-directory (/nix/store/[a-z0-9]{32}-horizon-os-model-normal-[a-f0-9]{12})', unit)
        if not match or '--qualification' in unit:
            raise RuntimeError('requires actual installed normal model service')
        lock = (Path(match[1]) / 'lock.json').read_bytes()
        if lock != (release / 'models/lock.json').read_bytes():
            raise RuntimeError('installed model lock differs from frozen source')
        argv = ['nix', 'develop', '--no-update-lock-file', '--no-write-lock-file', 'path:' + str(release),
                '--command', 'cargo', 'test', '--locked', '-p', 'aios-session', '--test', 'process_observer',
                'installed_process_task_persistent_bus_and_active_cancellation', '--', '--ignored', '--nocapture']
        with tempfile.TemporaryDirectory(prefix='aios-bus-process-task-') as directory:
            work = Path(directory)
            environment = {'HOME': os.environ['HOME'], 'PATH': '/run/current-system/sw/bin', 'LANG': 'C.UTF-8',
                           'CARGO_TARGET_DIR': str(work / 'target'), 'CARGO_HOME': str(work / 'cargo'),
                           'XDG_CACHE_HOME': str(work / 'cache'), 'TMPDIR': str(work)}
            result = subprocess.run(argv, cwd=release, env=environment, capture_output=True, timeout=600, check=False)
        if len(result.stdout) + len(result.stderr) > 1024 * 1024:
            raise RuntimeError('native bus task output exceeds bound')
        print(result.stdout.decode(errors='replace'), flush=True)
        print(result.stderr.decode(errors='replace'), flush=True)
        print('AIOS_BUS_PROCESS_TASK_COMMAND=' + json.dumps({'argv': argv, 'upstream_exit': result.returncode}), flush=True)
        if result.returncode or b'1 passed; 0 failed; 0 ignored' not in result.stdout:
            raise RuntimeError('native bus task qualification failed')
        prefix = b'AIOS_BUS_PROCESS_TASK='
        rows = [line[len(prefix):] for line in result.stdout.splitlines() if line.startswith(prefix)]
        if len(rows) != 1:
            raise RuntimeError('missing unique native bus task proof')
        proof = json.loads(rows[0])
        broker.verify()
        if snapshot.identity() != identity or proof['broker_pid'] != broker.identity['pid'] or proof['boot_id'] != identity['boot_id']:
            raise RuntimeError('native task broker/target differs from original installed service')
        require_answer(proof['bus_answer'], proof['native_identity'], proof['selection']['data']['process_id'])
        proof.update({'broker_identity': broker.identity, 'installed_executable': str(broker.binaries['aios-sessiond']),
                      'model_lock_sha256': hashlib.sha256(lock).hexdigest(), 'model_sha256': json.loads(lock)['artifact']['sha256']})
        print('AIOS_INSTALLED_BUS_PROCESS_TASK=' + json.dumps(proof, sort_keys=True), flush=True)
    finally:
        broker.close()


if __name__ == '__main__':
    main()
