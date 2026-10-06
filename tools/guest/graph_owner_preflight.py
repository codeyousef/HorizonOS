#!/usr/bin/env python3
"""Fixed initial root fixture for the original installed graph owner.

Disposable image only. No arguments, RPC, arbitrary commands, paths or units.
"""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import pwd
import re
import stat
import subprocess
import time

import desktop_probe
import snapshot

ROOT = Path('/var/lib/aios/state')
REPORT = Path('/run/aios-graph-acceptance/result.json')
PHASE = REPORT.with_name('phase.json')
UNIT = 'aios-state.service'
SYSTEMCTL = '/run/current-system/sw/bin/systemctl'
PROFILE = 'fixed-installed-graph-owner-v1'


def device_proof_valid(value, identity):
    try:
        row = value['selected']
        properties = row['properties']
        device = properties['device']
        native = value['native_properties']
        return (value['execution_authority'] is False and row['source_truth'] == 'running'
                and properties['execution_authority'] is False and properties['live_identity_retained'] is False
                and properties['captured']['boot'] == identity['boot_id']
                and device['syspath'] == value['native_syspath']
                and type(device['major']) is int and type(device['minor']) is int
                and [device['major'], device['minor']] == value['native_devnum']
                and device['serial'] == identity['disk_serial']
                and all(device[field] == native.get(key) for field, key in
                        [('serial', 'ID_SERIAL'), ('serial_short', 'ID_SERIAL_SHORT'), ('wwn', 'ID_WWN'),
                         ('bus', 'ID_BUS'), ('model', 'ID_MODEL'), ('vendor', 'ID_VENDOR')]))
    except (KeyError, TypeError):
        return False


def event_status_ready(observed, boot):
    try:
        return (observed['boot'] == boot and observed['event_watcher_installed'] is True
                and observed['event_watcher_error'] is None and observed['model_invoked'] is False
                and observed['execution_authority'] is False
                and all(type(observed[field]) is int and observed[field] >= 0
                        for field in ('systemd_notifications', 'event_reconciliations')))
    except (KeyError, TypeError):
        return False


def event_proof_valid(value, boot):
    try:
        before, after = value['before'], value['after']
        for observed in (before, after):
            if not event_status_ready(observed, boot):
                return False
        captured = value['service']['properties']['captured']
        previous = before['last_attempt']
        return (after['systemd_notifications'] > before['systemd_notifications']
                and after['event_reconciliations'] > before['event_reconciliations']
                and captured['boot'] == boot and previous['boot'] == boot
                and type(captured['monotonic_ns']) is int and type(previous['monotonic_ns']) is int
                and captured['monotonic_ns'] > previous['monotonic_ns']
                and all(value['service']['properties']['observation'][k] == value['native'][n]
                        for k, n in [('load_state', 'LoadState'), ('active_state', 'ActiveState'), ('sub_state', 'SubState')]))
    except (KeyError, TypeError):
        return False


def identity():
    value = snapshot.identity()
    authority = Path('/run/current-system/etc/aios/target-authority.json').resolve(strict=True)
    info = authority.stat()
    data = authority.read_bytes()
    expected = snapshot.decode(data)
    installed = Path('/etc/aios/target-authority.json')
    other = installed.stat()
    if (not authority.is_relative_to('/nix/store') or info.st_uid != 0 or info.st_mode & 0o222
            or not stat.S_ISREG(other.st_mode) or other.st_uid != 0 or other.st_mode & 0o222
            or installed.read_bytes() != data or value['os_id'] != 'nixos'
            or value['guest_role'] != 'development' or value['disk_serial'] != 'AIOS_DEV_ROOT'
            or value['management_channel'] != 'ssh-development'
            or value['dmi_uuid'] != Path('/sys/class/dmi/id/product_uuid').read_text().strip().lower()
            or any(value[k] != expected[k] for k in ('installation_uuid', 'dmi_uuid', 'guest_role', 'disk_serial', 'management_channel'))
            or Path('/etc/aios/graph-test-profile').read_text().strip() != PROFILE):
        raise RuntimeError('graph fixture target identity failed')
    return value


def recheck(expected):
    if identity() != expected:
        raise RuntimeError('graph fixture target changed')


def command(expected, argv):
    recheck(expected)
    result = subprocess.run(argv, capture_output=True, timeout=15)
    recheck(expected)
    if len(result.stdout) + len(result.stderr) > 131072:
        raise RuntimeError('graph fixture output bound exceeded')
    return result


def control(expected, operation):
    if operation not in ('start', 'stop', 'restart'):
        raise RuntimeError('unregistered graph fixture operation')
    result = command(expected, [SYSTEMCTL, operation, UNIT])
    if result.returncode:
        raise RuntimeError('fixed graph unit control failed')


def properties(expected, name, fields):
    if name not in (UNIT, 'sshd.service', 'aios-reconcile.service', 'aios-reconcile.timer', 'aios-model.service', 'aios-graph-denied-probe.service'):
        raise RuntimeError('unregistered graph fixture unit')
    result = command(expected, [SYSTEMCTL, 'show', name, *['--property=' + f for f in fields]])
    if result.returncode:
        raise RuntimeError('native unit inspection failed')
    return dict(line.split('=', 1) for line in result.stdout.decode().splitlines() if '=' in line)


def denied_probe_valid(value):
    # The immutable probe executes this exact package as dev. Require a real
    # exited process, not a start/namespace/credential failure or signal.
    return value == {'User': 'dev', 'Result': 'exit-code', 'ExecMainCode': '1',
                     'ExecMainStatus': '1', 'ActiveState': 'failed', 'SubState': 'failed'}


def denied_probe(expected):
    result = command(expected, [SYSTEMCTL, 'start', 'aios-graph-denied-probe.service'])
    value = properties(expected, 'aios-graph-denied-probe.service',
                       ['User', 'Result', 'ExecMainCode', 'ExecMainStatus', 'ActiveState', 'SubState'])
    if result.returncode == 0 or not denied_probe_valid(value):
        raise RuntimeError('normal development UID denial was not verified')
    return {'upstream_exit': 1, 'native_unit': value}


def raw_timer_trigger(data):
    fields = data.decode('ascii').strip().split()
    if len(fields) != 2 or fields[0] != 't' or not fields[1].isdigit():
        raise RuntimeError('native graph timer timestamp unavailable')
    trigger = int(fields[1])
    if trigger > (1 << 64) - 1:
        raise RuntimeError('native graph timer timestamp overflow')
    return trigger


def timer_properties(expected):
    # systemctl pretty-prints LastTriggerUSecMonotonic as a duration. Read the
    # fixed native typed property instead of reparsing localized display text.
    value = properties(expected, 'aios-reconcile.timer', ['ActiveState', 'ActiveEnterTimestampMonotonic'])
    argv = ['/run/current-system/sw/bin/busctl', '--address=unix:path=/run/dbus/system_bus_socket',
            'get-property', 'org.freedesktop.systemd1',
            '/org/freedesktop/systemd1/unit/aios_2dreconcile_2etimer',
            'org.freedesktop.systemd1.Timer', 'LastTriggerUSecMonotonic']
    result = command(expected, argv)
    if result.returncode:
        raise RuntimeError('native graph timer timestamp unavailable')
    value['LastTriggerUSecMonotonic'] = str(raw_timer_trigger(result.stdout))
    return value


def publish(path, value):
    info = path.parent.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0 or stat.S_IMODE(info.st_mode) != 0o755:
        raise RuntimeError('unsafe graph fixture report directory')
    data = snapshot.canonical(value)
    if len(data) > 131072:
        raise RuntimeError('graph fixture report bound exceeded')
    temporary = path.with_suffix('.tmp')
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'wb') as file:
        file.write(data); file.flush(); os.fsync(file.fileno())
        os.fchmod(file.fileno(), 0o444)
    os.replace(temporary, path)


def phase(expected, name):
    recheck(expected)
    publish(PHASE, {'schema_version': 1, 'identity': expected, 'phase': name,
                   'observed_monotonic_ns': time.monotonic_ns()})


def digest(path):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as file:
        before = os.fstat(file.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or before.st_size > 64 * 1024**2:
            raise RuntimeError('unsafe fixed graph fixture file')
        h = hashlib.sha256()
        while data := file.read(65536):
            h.update(data)
        after = os.fstat(file.fileno())
        if (before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (after.st_size, after.st_mtime_ns, after.st_ctime_ns):
            raise RuntimeError('fixed file changed during measurement')
        return {'bytes': before.st_size, 'sha256': h.hexdigest()}


def graph(expected, executable, *args):
    result = command(expected, [executable, *args])
    if result.returncode:
        raise RuntimeError('original installed graph control failed')
    value = snapshot.decode(result.stdout)
    if set(value) != {'ok', 'data'} or value['ok'] is not True:
        raise RuntimeError('invalid graph control response')
    return value['data']


def ready(expected, executable):
    deadline = time.monotonic() + 30
    while True:
        try:
            value = graph(expected, executable, '--inspect')
            if value['boot'] != expected['boot_id'] or value['scope'] != 'system' or value['period_seconds'] != 900:
                raise RuntimeError('graph status target differs')
            if value['model_invoked'] or value['execution_authority'] or value['reconciliations_attempted'] < 1:
                raise RuntimeError('graph owner status is invalid')
            if value['provider'] is not None and value['provider']['status'] in ('ready', 'partial'):
                return value
        except (RuntimeError, OSError):
            if time.monotonic() >= deadline:
                raise
        if time.monotonic() >= deadline:
            raise RuntimeError('native graph snapshot unavailable')
        time.sleep(0.25)


def installed_process(expected, executable):
    value = properties(expected, UNIT, ['MainPID', 'ActiveState', 'SubState', 'User', 'Group', 'NoNewPrivileges'])
    uid = pwd.getpwnam('aios-state').pw_uid
    if value['ActiveState'] != 'active' or value['SubState'] != 'running' or value['User'] != 'aios-state' or value['Group'] != 'aios-state':
        raise RuntimeError('original graph owner is not running')
    pid = int(value['MainPID']); proc = Path('/proc') / str(pid)
    status = dict(line.split(':', 1) for line in (proc / 'status').read_text().splitlines() if ':' in line)
    if (pid <= 0 or (proc / 'exe').resolve(strict=True) != Path(executable)
            or [int(x) for x in status['Uid'].split()] != [uid] * 4
            or any(int(status[k].strip(), 16) for k in ('CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb'))
            or status['NoNewPrivs'].strip() != '1'):
        raise RuntimeError('original graph owner executable/identity/capabilities differ')
    for path, mode in [(ROOT, 0o700), (Path('/run/aios-state'), 0o700), (ROOT / 'graph.sqlite3', 0o600), (Path('/run/aios-state/owner.sock'), 0o600)]:
        info = path.lstat()
        if info.st_uid != uid or stat.S_IMODE(info.st_mode) != mode or path.is_symlink():
            raise RuntimeError('graph private path permissions differ')
    return {'pid': pid, 'uid': uid, 'start_ticks': int((proc / 'stat').read_text().rsplit(')', 1)[1].split()[19]),
            'executable_sha256': digest(Path(executable))['sha256'], 'zero_capabilities': True, 'private_modes_verified': True}


def desktop(expected):
    recheck(expected)
    with contextlib.redirect_stdout(io.StringIO()) as output:
        desktop_probe.main()
    value = snapshot.decode(output.getvalue())
    if value['boot_id'] != expected['boot_id']:
        raise RuntimeError('desktop observation boot differs')
    recheck(expected)
    return value


def main():
    if os.getuid() != 0 or os.geteuid() != 0 or len(__import__('sys').argv) != 1:
        raise PermissionError('initial root fixture only')
    expected = identity()
    executable = os.environ['AIOS_GRAPH_EXECUTABLE']
    if not re.fullmatch(r'/nix/store/[a-z0-9]{32}-aios-state-0\.1\.0/bin/aios-stated', executable):
        raise RuntimeError('graph executable is not the compiled package')
    proof = {'schema_version': 1, 'identity': expected, 'evidence_kind': 'actual-installed-graph-owner', 'verified': False, 'steps': {}}
    try:
        phase(expected, 'startup')
        startup = ready(expected, executable)
        proof['steps']['startup'] = {'status': startup, 'process': installed_process(expected, executable)}
        deadline = time.monotonic() + 120
        while True:
            try:
                proof['steps']['startup_desktop'] = desktop(expected)
                break
            except (ValueError, subprocess.CalledProcessError):
                if time.monotonic() >= deadline:
                    raise
                time.sleep(1)
        # Boot reconciliation can precede SSH activation; sample natively now
        # before comparing mutable service state with the later live manager.
        graph(expected, executable, '--reconcile')
        generations = graph(expected, executable, '--generations')
        pointers = generations['data']['pointers']
        selected = Path('/nix/var/nix/profiles/system').resolve(strict=True)
        if (generations['execution_authority'] is not False or generations['data']['freshness'] != 'Current'
                or pointers['running_closure'] != expected['current_system']
                or pointers['selected_profile_closure'] != str(selected)
                or pointers['running_profile_divergence'] != (expected['current_system'] != str(selected))
                or pointers['bootloader_entry'] is not None or pointers['managed_transaction'] is not None):
            raise RuntimeError('native graph generation observations differ or infer unobserved provenance')
        proof['steps']['startup_generations'] = generations
        devices = graph(expected, executable, '--devices')
        # Independent fixed managed QEMU root-disk locator, never a graph/model
        # path interpreted with root authority or a raw block-device read.
        native_path = Path('/sys/class/block/vda').resolve(strict=True)
        selected = next(row for row in devices['data'] if row['properties']['device']['syspath'] == str(native_path))
        independent = command(expected, ['/run/current-system/sw/bin/udevadm', 'info', '--query=property', '--path', str(native_path)])
        if independent.returncode:
            raise RuntimeError('fixed native root disk property inspection failed')
        native_properties = dict(line.split('=', 1) for line in independent.stdout.decode().splitlines() if '=' in line)
        device_proof = {'selected': selected, 'native_syspath': str(native_path),
            'native_devnum': [int(v) for v in Path('/sys/class/block/vda/dev').read_text().strip().split(':')],
            'native_properties': {key: native_properties[key] for key in
                ('ID_SERIAL', 'ID_SERIAL_SHORT', 'ID_WWN', 'ID_BUS', 'ID_MODEL', 'ID_VENDOR') if native_properties.get(key)},
            'execution_authority': devices['execution_authority']}
        if not device_proof_valid(device_proof, expected):
            raise RuntimeError('native graph root disk properties differ or infer missing identity')
        proof['steps']['startup_devices'] = device_proof
        cached = graph(expected, executable, '--service', 'sshd.service')
        native = properties(expected, 'sshd.service', ['LoadState', 'ActiveState', 'SubState'])
        observed = cached['data']['properties']['observation']
        if any(observed[k] != native[n] for k, n in [('load_state', 'LoadState'), ('active_state', 'ActiveState'), ('sub_state', 'SubState')]):
            raise RuntimeError('cached graph SSH observation differs from independent native systemd')
        proof['steps']['service_comparison'] = {'native': native, 'captured': cached['data']['properties']['captured'], 'unknown_properties_preserved': all(cached['data']['properties'][k] is None for k in ('main_pid', 'result', 'ordering_after'))}
        if not proof['steps']['service_comparison']['unknown_properties_preserved']:
            raise RuntimeError('unobserved service properties were invented')
        # Boot notifications can deliberately exhaust the bounded drain and
        # disconnect the watcher. Require recovery BEFORE taking the baseline
        # and starting the fixed probe; older boot counters cannot prove it.
        baseline_deadline = time.monotonic() + 30
        while True:
            event_before = graph(expected, executable, '--inspect')
            if event_status_ready(event_before, expected['boot_id']):
                break
            if time.monotonic() >= baseline_deadline:
                proof['failed_systemd_event_baseline'] = event_before
                raise RuntimeError('native systemd watcher did not recover before the event probe')
            time.sleep(0.2)
        proof['steps']['foreign_uid_denied'] = denied_probe(expected)
        deadline = time.monotonic() + 15
        while True:
            event_after = graph(expected, executable, '--inspect')
            event_proof = {'before': event_before, 'after': event_after,
                'service': graph(expected, executable, '--service', 'sshd.service')['data'],
                'native': properties(expected, 'sshd.service', ['LoadState', 'ActiveState', 'SubState'])}
            if event_proof_valid(event_proof, expected['boot_id']):
                proof['steps']['systemd_events'] = event_proof
                break
            if time.monotonic() >= deadline:
                # Preserve the last native counters and independent comparison
                # on failure. This diagnostic cannot satisfy the required step.
                proof['failed_systemd_event_observation'] = event_proof
                raise RuntimeError('actual native systemd notification and snapshot refresh were not verified')
            time.sleep(0.2)
        before = proof['steps']['startup']['process']
        control(expected, 'restart'); ready(expected, executable)
        after = installed_process(expected, executable)
        if (after['pid'], after['start_ticks']) == (before['pid'], before['start_ticks']):
            raise RuntimeError('graph owner did not restart')
        proof['steps']['restart'] = {'before': before, 'after': after}
        control(expected, 'stop'); phase(expected, 'graph_outage')
        proof['steps']['outage_desktop'] = desktop(expected)
        proof['steps']['outage_ssh'] = properties(expected, 'sshd.service', ['ActiveState', 'SubState'])
        if proof['steps']['outage_ssh'] != {'ActiveState': 'active', 'SubState': 'running'}:
            raise RuntimeError('SSH failed during graph outage')
        time.sleep(8)
        ledger = Path('/var/lib/aios/transactions/ledger.sqlite')
        ledger_before = digest(ledger)
        database = ROOT / 'graph.sqlite3'
        info = database.lstat(); uid = pwd.getpwnam('aios-state').pw_uid
        if not stat.S_ISREG(info.st_mode) or info.st_uid != uid or stat.S_IMODE(info.st_mode) != 0o600:
            raise RuntimeError('unsafe graph corruption fixture target')
        fd = os.open(database, os.O_RDWR | os.O_NOFOLLOW)
        try:
            opened = os.fstat(fd)
            if (opened.st_dev, opened.st_ino) != (info.st_dev, info.st_ino):
                raise RuntimeError('graph fixture inode changed')
            os.pwrite(fd, b'X', 0); os.fsync(fd)
        finally:
            os.close(fd)
        corrupted = digest(database)
        control(expected, 'start'); recovered = ready(expected, executable)
        name = recovered['quarantine']
        if not isinstance(name, str) or not re.fullmatch(r'graph\.quarantine-[0-9a-f]{32}', name):
            raise RuntimeError('installed graph did not quarantine corruption')
        archive = ROOT / name
        if digest(archive / 'graph.sqlite3') != corrupted or database.stat().st_ino == info.st_ino:
            raise RuntimeError('quarantine lost corrupted bytes or reused old inode')
        receipt = snapshot.decode((archive / 'receipt.json').read_bytes())
        if digest(ledger) != ledger_before:
            raise RuntimeError('transaction ledger changed during graph rebuild')
        proof['steps']['corruption_recovery'] = {'corrupted': corrupted, 'quarantine': name, 'receipt': receipt, 'ledger_unchanged': True, 'process': installed_process(expected, executable)}
        phase(expected, 'waiting_real_timer')
        timer_before = timer_properties(expected)
        if timer_before['ActiveState'] != 'active':
            raise RuntimeError('installed graph timer is inactive')
        started = time.monotonic_ns(); deadline = started + 1030_000_000_000
        while True:
            recheck(expected)
            elapsed = time.monotonic_ns() - started
            timer = timer_properties(expected)
            last = int(timer['LastTriggerUSecMonotonic'])
            baseline = int(timer_before['LastTriggerUSecMonotonic'])
            anchor = baseline or int(timer_before['ActiveEnterTimestampMonotonic'])
            status = graph(expected, executable, '--inspect')
            oneshot = properties(expected, 'aios-reconcile.service', ['Result', 'ExecMainStatus', 'ExecMainStartTimestampMonotonic'])
            if (elapsed >= 900_000_000_000 and last > baseline and last - anchor >= 899_000_000
                    and status['reconciliations_attempted'] > recovered['reconciliations_attempted']
                    and oneshot['Result'] == 'success' and oneshot['ExecMainStatus'] == '0'
                    and int(oneshot['ExecMainStartTimestampMonotonic']) >= last):
                proof['steps']['real_timer'] = {'elapsed_ns': elapsed, 'before': timer_before, 'after': timer, 'oneshot': oneshot, 'status': status}
                break
            if time.monotonic_ns() >= deadline:
                raise RuntimeError('actual fifteen-minute reconciliation was not verified')
            time.sleep(2)
        proof['steps']['final_desktop'] = desktop(expected)
        proof['steps']['final_process'] = installed_process(expected, executable)
        proof['verified'] = True
        phase(expected, 'complete')
        publish(REPORT, proof)
    except Exception as error:
        proof['error_class'] = type(error).__name__
        if isinstance(error, RuntimeError):
            # All RuntimeError messages in this fixed fixture are static source
            # diagnostics, never provider payloads, command output or documents.
            proof['error_reason'] = str(error)
        publish(REPORT, proof)
        raise


if __name__ == '__main__':
    main()
