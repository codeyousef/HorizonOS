#!/usr/bin/env python3
"""Fixed root-only disposable-image model crash/restart measurement.

No RPC, arguments, arbitrary units, commands, paths or model-selected operation.
The production model receives no authorization to invoke this fixture.
"""
import hashlib
import json
import os
from pathlib import Path
import pwd
import stat
import subprocess
import sys
import time

from model_service_smoke import Client
import snapshot

REPORT = Path('/run/aios-model-acceptance/result.json')
UNIT = 'aios-model.service'
SYSTEMCTL = '/run/current-system/sw/bin/systemctl'


def identity():
    value = snapshot.identity()
    authority = Path('/etc/aios/target-authority.json').resolve(strict=True)
    info = authority.stat()
    expected = json.loads(authority.read_text())
    actual_dmi = Path('/sys/class/dmi/id/product_uuid').read_text().strip().lower()
    if (not authority.is_relative_to('/nix/store') or info.st_uid != 0 or info.st_mode & 0o222
            or value['os_id'] != 'nixos' or value['guest_role'] != 'development'
            or value['dmi_uuid'] != actual_dmi or value['disk_serial'] != 'AIOS_DEV_ROOT'
            or Path('/etc/aios/management-channel').read_text().strip() != 'ssh-development'
            or any(value[k] != expected[k] for k in ('installation_uuid', 'dmi_uuid', 'guest_role', 'disk_serial', 'management_channel'))):
        raise RuntimeError('fixed lifecycle target identity failed')
    return value


def recheck(expected):
    if identity() != expected:
        raise RuntimeError('model acceptance target changed')


def control(expected, *operation):
    recheck(expected)
    argv = [SYSTEMCTL, *operation, UNIT]
    result = subprocess.run(argv, capture_output=True, text=True, timeout=10)
    recheck(expected)
    if result.returncode != 0 or len(result.stdout) + len(result.stderr) > 65536:
        raise RuntimeError('fixed model unit control failed')
    return {'argv': argv, 'upstream_exit': result.returncode}


def unit(expected):
    recheck(expected)
    result = subprocess.run([SYSTEMCTL, 'show', UNIT, '--property=MainPID', '--property=NRestarts',
                             '--property=ActiveState', '--property=SubState', '--property=ControlGroup'],
                            capture_output=True, text=True, check=True, timeout=10)
    recheck(expected)
    return dict(line.split('=', 1) for line in result.stdout.splitlines() if '=' in line)


def process(expected):
    observed = unit(expected)
    if observed['ActiveState'] != 'active' or observed['ControlGroup'] != '/system.slice/aios-model.service':
        raise RuntimeError('model unit is not the expected installed process')
    pid = int(observed['MainPID'])
    if pid <= 1:
        raise RuntimeError('model process missing')
    directory = Path('/proc') / str(pid)
    executable = Path('/run/current-system/sw/bin/aios-modeld').resolve(strict=True)
    if (directory / 'exe').resolve(strict=True) != executable:
        raise RuntimeError('unit main process differs from installed model')
    status = dict(line.split(':', 1) for line in (directory / 'status').read_text().splitlines() if ':' in line)
    if any(int(uid) != pwd.getpwnam('aios-model').pw_uid for uid in status['Uid'].split()):
        raise RuntimeError('model process identity mismatch')
    rollup = dict(line.split(':', 1) for line in (directory / 'smaps_rollup').read_text().splitlines() if ':' in line)
    sample = {'pid': pid, 'unit': observed, 'start_ticks': (directory / 'stat').read_text().split(') ', 1)[1].split()[19],
              'pss_bytes': int(rollup['Pss'].split()[0]) * 1024,
              'vm_hwm_bytes': int(status['VmHWM'].split()[0]) * 1024,
              'executable': str(executable), 'executable_sha256': hashlib.sha256(executable.read_bytes()).hexdigest()}
    if unit(expected) != observed:
        raise RuntimeError('model changed during process observation')
    return sample


def status(client):
    result = client.call({'kind': 'get_status'})
    if result['error']:
        raise RuntimeError('actual model status failed')
    proof = result['data']['isolation']
    required = {'execve', 'execveat', 'ipv4_socket', 'ipv6_socket', 'home', 'root_home', 'user_runtime_keyring',
                'system_logs', 'runtime_logs', 'system_bus', 'nix_daemon', 'model_manifest_write', 'runtime_configuration_write'}
    if (proof['schema_version'] != 1 or proof['evidence_kind'] != 'actual-installed-kernel-denials'
            or len(proof['checks']) != len(required) or {c['boundary'] for c in proof['checks']} != required):
        raise RuntimeError('model has no actual complete installed kernel denial proof')
    return result['data']


def generate(client, long=False):
    text = ('Write a very long answer listing the numbers 1 to 10000.' if long else 'Observation ev_lifecycle: the operating system is NixOS. What OS is reported?')
    result = client.call({'kind': 'generate', 'generation': {'profile': 'normal',
                         'system_prompt': 'Return an answer JSON object. Use only the observation and cite ev_lifecycle. Perform no actions.',
                         'user_prompt': text, 'response_mode': 'final_answer', 'allowed_tools': [],
                         'evidence_ids': ['ev_lifecycle'], 'deadline_ms': 90000}})
    if result['error']:
        raise RuntimeError('fixed lifecycle generation failed')
    return result['data']['generation_id']


def main():
    if os.getuid() != 0 or os.geteuid() != 0 or len(sys.argv) != 1:
        return 5
    if Path('/etc/aios/model-test-profile').read_text().strip() != 'installed-normal-cpu-model-v1':
        return 5
    parent = REPORT.parent.stat()
    if not stat.S_ISDIR(parent.st_mode) or parent.st_uid != 0 or parent.st_mode & 0o022 or REPORT.exists():
        return 8
    expected = identity()
    report = {'schema_version': 1, 'evidence_kind': 'actual-installed-model-fixed-root-lifecycle-fixture',
              'identity': expected, 'verified': False, 'controls': [], 'restarts': [], 'memory_samples': [],
              'limits': ['Root-only initial acceptance instrumentation; absent from production composition.',
                         'PSS observations are samples, not a true process peak or all-buffer forensic proof.',
                         'Direct inference transport loss is observed; user-facing MODEL_CRASHED qualification is separate.']}
    client = None
    try:
        client = Client('/run/aios/model.sock')
        report['initial_status'] = status(client)
        answer = client.wait(generate(client))
        if (answer['state'] != 'completed' or answer['output']['evidence_ids'] != ['ev_lifecycle']
                or 'NixOS' not in answer['output']['text'] or answer['mutation_performed']):
            raise RuntimeError('actual initial answer failed')
        report['initial_answer'] = answer
        report['memory_samples'].append({'phase': 'loaded_after_context_teardown', **process(expected)})
        response = client.call({'kind': 'unload'})
        if response['error']:
            raise RuntimeError('model explicit unload failed')
        deadline = time.monotonic() + 10
        while status(client)['loaded']:
            if time.monotonic() > deadline:
                raise RuntimeError('model explicit unload did not finish')
            time.sleep(0.05)
        report['memory_samples'].append({'phase': 'unloaded', **process(expected)})
        active = generate(client, long=True)
        deadline = time.monotonic() + 10
        while not status(client)['busy']:
            if time.monotonic() > deadline:
                raise RuntimeError('model active request did not start')
            time.sleep(0.01)
        for iteration in range(4):
            before = process(expected)
            began = time.monotonic()
            report['controls'].append(control(expected, 'kill', '--kill-whom=main', '--signal=KILL'))
            if iteration == 0:
                try:
                    client.result(active)
                except (RuntimeError, OSError):
                    report['active_request_transport_lost'] = True
                else:
                    raise RuntimeError('crashed process returned a result')
            client.socket.close(); client = None
            deadline = began + 40
            while True:
                observed = unit(expected)
                if observed['ActiveState'] == 'active' and int(observed['MainPID']) > 1 and observed['MainPID'] != str(before['pid']):
                    break
                if time.monotonic() > deadline:
                    raise RuntimeError('actual model restart did not occur')
                time.sleep(0.05)
            after = process(expected)
            delay = time.monotonic() - began
            if (int(after['unit']['NRestarts']) != int(before['unit']['NRestarts']) + 1 or delay < 1.8
                    or iteration and delay < report['restarts'][-1]['delay_seconds'] * 1.3):
                raise RuntimeError('actual restart counter/backoff did not increase')
            client = Client('/run/aios/model.sock')
            after_status = status(client)
            if after_status['loaded'] or after_status['busy'] or after_status['own_queued']:
                raise RuntimeError('restarted model inherited request or model state')
            report['restarts'].append({'before': before, 'after': after, 'delay_seconds': round(delay, 3), 'status': after_status})
        recovered = client.wait(generate(client))
        if recovered['state'] != 'completed' or recovered['output']['evidence_ids'] != ['ev_lifecycle'] or recovered['mutation_performed']:
            raise RuntimeError('model did not answer after restart recovery')
        report['recovered_answer'] = recovered
        recheck(expected)
        report['verified'] = True
    except (OSError, ValueError, RuntimeError, KeyError, subprocess.SubprocessError) as error:
        report['failure_type'] = type(error).__name__
        report['failure'] = str(error)
    finally:
        if client is not None:
            client.socket.close()
        descriptor = os.open(REPORT, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o644)
        with os.fdopen(descriptor, 'w') as output:
            json.dump(report, output, sort_keys=True); output.write('\n'); output.flush(); os.fsync(output.fileno())
        os.chmod(REPORT, 0o644)
    return 0 if report['verified'] else 8


if __name__ == '__main__':
    raise SystemExit(main())
