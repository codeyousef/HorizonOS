#!/usr/bin/env python3
"""Fixed root-only disposable-image model crash/restart measurement.

No RPC, arguments, arbitrary units, commands, paths or model-selected operation.
The production model receives no authorization to invoke this fixture.
"""
import hashlib
import contextlib
import io
import json
import os
from pathlib import Path
import pwd
import stat
import subprocess
import sys
import time

from model_service_smoke import Client
import model_queue_fixture
from model_public_fixture import PublicProbe, require_failure
import desktop_probe
import snapshot

REPORT = Path('/run/aios-model-acceptance/result.json')
UNIT = 'aios-model.service'
SYSTEMCTL = '/run/current-system/sw/bin/systemctl'
PHASE = REPORT.parent / 'phase.json'
DROPIN = Path('/run/systemd/system/aios-model.service.d/99-aios-model-corruption.conf')


def phase(expected, name):
    recheck(expected)
    value = {'schema_version': 1, 'evidence_kind': 'actual-fixed-model-failure-phase', 'identity': expected,
             'phase': name, 'model_unit': unit(expected), 'observed_boottime_ns': time.clock_gettime_ns(time.CLOCK_BOOTTIME)}
    temporary = PHASE.with_suffix('.tmp')
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'w') as output:
        json.dump(value, output); output.flush(); os.fsync(output.fileno())
    os.chmod(temporary, 0o644)
    os.replace(temporary, PHASE)
    recheck(expected)


def desktop(expected):
    recheck(expected)
    with contextlib.redirect_stdout(io.StringIO()) as output:
        desktop_probe.main()
    value = json.loads(output.getvalue())
    if value['boot_id'] != expected['boot_id']:
        raise RuntimeError('desktop continuity target changed')
    recheck(expected)
    return value


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as file:
        while data := file.read(1024 * 1024):
            h.update(data)
    return h.hexdigest()


def reload(expected):
    recheck(expected)
    result = subprocess.run([SYSTEMCTL, 'daemon-reload'], capture_output=True, timeout=10)
    recheck(expected)
    if result.returncode:
        raise RuntimeError('fixed test mount reload failed')


def corrupt_model(expected, public):
    if os.getuid() != 0 or os.geteuid() != 0:
        raise PermissionError('corruption coordinator requires initial root fixture')
    recheck(expected)
    profile = Path('/etc/aios/model-corruption-test.json').resolve(strict=True)
    info = profile.stat()
    if not profile.is_relative_to('/nix/store') or info.st_uid != 0 or info.st_mode & 0o222:
        raise RuntimeError('corruption profile is not immutable compiled test data')
    data = snapshot.decode(profile.read_bytes())
    if set(data) != {'schema_version', 'original', 'corrupt', 'bytes', 'expected_sha256', 'corrupt_sha256'} or data['schema_version'] != 1:
        raise RuntimeError('invalid compiled corruption profile')
    original, bad = Path(data['original']), Path(data['corrupt'])
    for path in (original, bad):
        info = path.lstat()
        if (path.resolve(strict=True) != path or not path.is_relative_to('/nix/store') or not stat.S_ISREG(info.st_mode)
                or info.st_uid != 0 or info.st_mode & 0o222 or info.st_size != data['bytes']):
            raise RuntimeError('corruption fixture artifact is not protected immutable data')
    if digest(original) != data['expected_sha256'] or digest(bad) != data['corrupt_sha256'] or data['corrupt_sha256'] == data['expected_sha256']:
        raise RuntimeError('immutable corruption fixture digests differ')
    parent = DROPIN.parent.lstat()
    if not stat.S_ISDIR(parent.st_mode) or parent.st_uid != 0 or stat.S_IMODE(parent.st_mode) != 0o700 or DROPIN.exists() or DROPIN.is_symlink():
        raise RuntimeError('fixed corruption drop-in directory is unsafe')
    payload = ('[Service]\nBindReadOnlyPaths=' + str(bad) + ':' + str(original) + ':norbind\n').encode()
    control(expected, 'stop')
    control(expected, 'reset-failed')
    fd = os.open(DROPIN, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'wb') as output:
        output.write(payload); output.flush(); os.fsync(output.fileno())
        owned = os.fstat(output.fileno())
    try:
        reload(expected)
        control(expected, 'start')
        before = process(expected)
        # Observe the actual running daemon's mount view, never alter store data.
        actual_path = Path('/proc') / str(before['pid']) / 'root' / str(original).lstrip('/')
        actual_sha = digest(actual_path)
        if actual_sha != data['corrupt_sha256'] or digest(original) != data['expected_sha256'] or process(expected)['start_ticks'] != before['start_ticks']:
            raise RuntimeError('actual daemon did not see only the immutable one-byte corrupt view')
        public.call('start_short')
        rejected = public.call('result')
        require_failure(rejected, 'TARGET_CHANGED')
        probe = Client('/run/aios/model.sock')
        try:
            rejected_status = status(probe)
        finally:
            probe.socket.close()
        if rejected_status['loaded'] or rejected_status['busy'] or rejected_status['own_queued']:
            raise RuntimeError('hash-mismatched model retained inference state')
        phase(expected, 'corrupt_model')
        health = public.call('health')
        desktop_proof = desktop(expected)
        # Give the registered host runner time to perform actual pinned SSH
        # reads in this failure phase; the data view remains corrupt throughout.
        time.sleep(4)
        return {'verified': True, 'evidence_kind': 'actual-installed-production-daemon-immutable-hash-mismatch',
                'artifact': data, 'actual_daemon_view_sha256': actual_sha, 'process': before, 'public_failure': rejected,
                'status_after_rejection': rejected_status, 'deterministic_and_kwin_health': health, 'desktop': desktop_proof}
    finally:
        control(expected, 'stop')
        info = DROPIN.lstat()
        if (info.st_dev, info.st_ino, info.st_uid, stat.S_IMODE(info.st_mode)) != (owned.st_dev, owned.st_ino, 0, 0o600) or DROPIN.read_bytes() != payload:
            raise RuntimeError('fixed corruption drop-in changed; cleanup refused')
        DROPIN.unlink()
        reload(expected)
        control(expected, 'reset-failed')
        control(expected, 'start')
        phase(expected, 'restored')


def identity():
    value = snapshot.identity()
    authority = Path('/run/current-system/etc/aios/target-authority.json').resolve(strict=True)
    info = authority.stat()
    data = authority.read_bytes()
    installed = Path('/etc/aios/target-authority.json')
    installed_info = installed.stat()
    expected = json.loads(data)
    actual_dmi = Path('/sys/class/dmi/id/product_uuid').read_text().strip().lower()
    if (not authority.is_relative_to('/nix/store') or info.st_uid != 0 or info.st_mode & 0o222
            or not stat.S_ISREG(installed_info.st_mode) or installed_info.st_uid != 0 or installed_info.st_mode & 0o222
            or installed.read_bytes() != data
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
    report = {'schema_version': 1, 'evidence_kind': 'actual-installed-model-fixed-root-lifecycle-fixture',
              'identity': None, 'verified': False, 'controls': [], 'restarts': [], 'memory_samples': [],
              'limits': ['Root-only initial acceptance instrumentation; absent from production composition.',
                         'PSS observations are samples, not a true process peak or all-buffer forensic proof.',
                         'Finite desktop/SSH failure-phase samples do not prove continuous or latency-qualified availability.']}
    client = None
    public = None
    try:
        expected = identity()
        report['identity'] = expected
        client = Client('/run/aios/model.sock')
        report['initial_status'] = status(client)
        answer = client.wait(generate(client))
        if (answer['state'] != 'completed' or answer['output']['evidence_ids'] != ['ev_lifecycle']
                or 'NixOS' not in answer['output']['text'] or answer['mutation_performed']):
            raise RuntimeError('actual initial answer failed')
        report['initial_answer'] = answer
        report['memory_samples'].append({'phase': 'loaded_after_context_teardown', **process(expected)})
        client.socket.close(); client = None
        report['queue_load'] = model_queue_fixture.run(expected, recheck, process)
        client = Client('/run/aios/model.sock')
        response = client.call({'kind': 'unload'})
        if response['error']:
            raise RuntimeError('model explicit unload failed')
        deadline = time.monotonic() + 10
        while status(client)['loaded']:
            if time.monotonic() > deadline:
                raise RuntimeError('model explicit unload did not finish')
            time.sleep(0.05)
        report['memory_samples'].append({'phase': 'unloaded', **process(expected)})
        # Type=exec lets the graphical target finish independently of this test.
        ready_by = time.monotonic() + 30
        while True:
            try:
                report['desktop_before_crashes'] = desktop(expected)
                break
            except (ValueError, subprocess.SubprocessError):
                if time.monotonic() > ready_by:
                    raise RuntimeError('actual desktop did not become ready')
                time.sleep(0.1)
        public = PublicProbe(expected, recheck)
        report['public_client'] = public.proof
        report['public_cli_start'] = public.call('start_crash')
        deadline = time.monotonic() + 10
        while True:
            observed = status(client)
            if observed['busy'] and observed['loaded']:
                break
            if time.monotonic() > deadline:
                raise RuntimeError('model active request did not start')
            time.sleep(0.01)
        for iteration in range(4):
            before = process(expected)
            began = time.monotonic()
            report['controls'].append(control(expected, 'kill', '--kill-whom=main', '--signal=KILL'))
            phase(expected, 'crash_outage')
            if iteration == 0:
                report['public_crash_failure'] = public.call('result')
                require_failure(report['public_crash_failure'], 'MODEL_CRASHED')
                try:
                    status(client)
                except (RuntimeError, OSError):
                    report['active_request_transport_lost'] = True
                else:
                    raise RuntimeError('crashed process returned a result')
            during = unit(expected)
            health = public.call('health')
            report.setdefault('crash_continuity', []).append({'model_unit': during, 'health': health, 'desktop': desktop(expected)})
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
            phase(expected, 'restored')
        recovered = client.wait(generate(client))
        if recovered['state'] != 'completed' or recovered['output']['evidence_ids'] != ['ev_lifecycle'] or recovered['mutation_performed']:
            raise RuntimeError('model did not answer after restart recovery')
        report['recovered_answer'] = recovered
        client.socket.close(); client = None
        report['corrupt_model'] = corrupt_model(expected, public)
        public.call('start_short')
        restored = public.call('result')
        # Retain the actual response even when the recovery gate fails.
        report['public_answer_after_corrupt_view_restored'] = restored
        answer = restored['answer']
        if (restored['upstream_exit'] != 0 or answer['state'] != 'completed' or answer['error'] is not None
                or answer['mutation_performed'] or 'NixOS' not in answer['output']['response']['text']):
            raise RuntimeError('public model did not recover after restoring the immutable view')
        report['desktop_after_failures'] = desktop(expected)
        if report['desktop_before_crashes']['processes'] != report['desktop_after_failures']['processes']:
            raise RuntimeError('desktop processes restarted during model failures')
        public.close(); public = None
        recheck(expected)
        report['verified'] = True
    except (OSError, ValueError, RuntimeError, KeyError, subprocess.SubprocessError) as error:
        report['failure_type'] = type(error).__name__
        report['failure'] = str(error)
    finally:
        if client is not None:
            client.socket.close()
        if public is not None:
            try:
                public.close()
            except (OSError, ValueError, RuntimeError, KeyError, subprocess.SubprocessError) as error:
                report['verified'] = False
                report['cleanup_failure_type'] = type(error).__name__
                report['cleanup_failure'] = str(error)
        descriptor = os.open(REPORT, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o644)
        with os.fdopen(descriptor, 'w') as output:
            json.dump(report, output, sort_keys=True); output.write('\n'); output.flush(); os.fsync(output.fileno())
        os.chmod(REPORT, 0o644)
    return 0 if report['verified'] else 8


if __name__ == '__main__':
    raise SystemExit(main())
