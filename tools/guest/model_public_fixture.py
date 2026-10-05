"""Fixed installed public-client failure checks, invoked only by initial fixture.

The coordinator is root; its client permanently becomes tester before any IPC.
No public RPC, arbitrary operation or replacement broker binary is provided.
"""
import grp
import hashlib
import json
import os
from pathlib import Path
import pwd
import re
import socket
import struct
import stat
import subprocess
import uuid

from model_queue_fixture import attest, cleanup, drop_ids, receive, send


class Broker:
    def __init__(self, uid=1001):
        if uid not in (1000, 1001) or os.getuid() != uid or os.geteuid() != uid:
            raise PermissionError('public fixture must first drop to tester')
        self.uid = uid
        self.runtime = Path('/run/user') / str(uid)
        info = self.runtime.lstat()
        if (self.runtime.resolve() != self.runtime or not stat.S_ISDIR(info.st_mode)
                or info.st_uid != self.uid or info.st_mode & 0o077):
            raise RuntimeError('public fixture runtime identity failed')
        self.env = {**os.environ, 'HOME': pwd.getpwuid(uid).pw_dir, 'PATH': '/run/current-system/sw/bin',
                    'XDG_RUNTIME_DIR': str(self.runtime), 'DBUS_SESSION_BUS_ADDRESS': 'unix:path=' + str(self.runtime / 'bus')}
        self.unit = 'aios-sessiond.service'
        self.binaries = {}
        for name, package in (('aios-sessiond', 'aios-core'), ('aiosctl', 'aios-cli')):
            binary = Path('/run/current-system/sw/bin', name).resolve(strict=True)
            info = binary.stat()
            if (not re.fullmatch(r'/nix/store/[a-z0-9]{32}-' + package + r'-[A-Za-z0-9._+-]+/bin/' + name, str(binary))
                    or not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o222 or not info.st_mode & 0o111):
                raise RuntimeError('public fixture requires exact protected installed binaries')
            self.binaries[name] = binary
        self.bytes = (self.binaries['aios-sessiond'].parents[1] / 'share/systemd/user/aios-sessiond.service').read_bytes()
        if not 0 < len(self.bytes) <= 65536:
            raise RuntimeError('installed broker unit exceeds bound')
        self.cli = None
        self.proof = self.show()
        self.identity = self.verify()

    @staticmethod
    def protected_file(path):
        resolved = Path(path).resolve(strict=True)
        info = resolved.stat()
        if (not re.fullmatch(r'/nix/store/[a-z0-9]{32}-[^/]+/.+', str(resolved))
                or not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o222):
            raise RuntimeError('broker unit is not protected installed store content')
        return resolved

    def verify(self):
        current = self.show()
        expected = {'NoNewPrivileges': 'yes', 'PrivateNetwork': 'yes', 'ProtectHome': 'tmpfs', 'ProtectSystem': 'strict',
                    'MemoryMax': '268435456', 'TasksMax': '64', 'RuntimeDirectoryMode': '0700', 'ActiveState': 'active'}
        if any(current.get(k) != v for k, v in expected.items()):
            raise RuntimeError('original installed broker hardening failed')
        fragment = self.protected_file(current['FragmentPath'])
        if fragment.read_bytes() != self.bytes:
            raise RuntimeError('original installed broker unit changed')
        for path in current['DropInPaths'].split():
            self.protected_file(path)
        executable = str(self.binaries['aios-sessiond'])
        # systemctl's fixed single-command representation; arguments or a
        # replacement executable are rejected even if FragmentPath is genuine.
        prefix = '{ path=' + executable + ' ; argv[]=' + executable + ' ; ignore_errors=no ; '
        if not current['ExecStart'].startswith(prefix) or current['ExecStart'].count('{ path=') != 1:
            raise RuntimeError('original broker ExecStart changed')
        pid = int(current['MainPID'])
        if pid <= 1 or self.bus('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus',
                                'GetConnectionUnixProcessID', 's', 'org.aios.Session1').strip() != f'u {pid}'.encode():
            raise RuntimeError('installed broker bus owner changed')
        def bus_identity(name, method):
            value = self.bus('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', method, 's', name)
            if not re.fullmatch(rb'u [0-9]+\n?', value):
                raise RuntimeError('invalid native bus credential response')
            return int(value.split()[1])
        if bus_identity('org.aios.Session1', 'GetConnectionUnixUser') != self.uid:
            raise RuntimeError('original broker UID changed')
        manager = bus_identity('org.freedesktop.systemd1', 'GetConnectionUnixProcessID')
        if bus_identity('org.freedesktop.systemd1', 'GetConnectionUnixUser') != self.uid:
            raise RuntimeError('user manager UID changed')
        system_env = {**self.env, 'DBUS_SYSTEM_BUS_ADDRESS': 'unix:path=/run/dbus/system_bus_socket'}
        root_view = subprocess.run(['/run/current-system/sw/bin/systemctl', '--system', 'show', f'user@{self.uid}.service',
                                    '--property=MainPID', '--value'], env=system_env, check=True,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5)
        if root_view.stdout.strip() != str(manager).encode():
            raise RuntimeError('user manager is not associated with root manager')
        ticks = (Path('/proc') / str(pid) / 'stat').read_text().rsplit(') ', 1)[1].split()[19]
        identity = {'pid': pid, 'start_ticks': ticks, 'manager_pid': manager,
                    'invocation_id': current['InvocationID'],
                    'boot_id': Path('/proc/sys/kernel/random/boot_id').read_text().strip()}
        if not re.fullmatch(r'[0-9a-f]{32}', identity['invocation_id']):
            raise RuntimeError('broker invocation identity missing')
        if hasattr(self, 'identity') and identity != self.identity:
            raise RuntimeError('original broker process identity changed')
        return identity

    def ctl(self, *arguments):
        return subprocess.run(['/run/current-system/sw/bin/systemctl', '--user', *arguments], env=self.env,
                              check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=12)

    def bus(self, *arguments):
        result = subprocess.run(['/run/current-system/sw/bin/busctl', '--user', 'call', *arguments], env=self.env,
                                check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5)
        if len(result.stdout) > 65536:
            raise RuntimeError('fixed desktop bus response exceeded bound')
        return result.stdout

    def show(self):
        names = ('ActiveState', 'MainPID', 'FragmentPath', 'NoNewPrivileges', 'PrivateNetwork', 'ProtectHome',
                 'ProtectSystem', 'MemoryMax', 'TasksMax', 'RuntimeDirectoryMode', 'DropInPaths', 'ExecStart', 'InvocationID')
        value = self.ctl('show', self.unit, *['--property=' + name for name in names]).stdout.decode()
        return dict(line.split('=', 1) for line in value.splitlines() if '=' in line)

    def start(self, long):
        self.verify()
        if self.cli is not None:
            raise RuntimeError('public fixture already has a CLI')
        question = ('word ' * 4500 if long else '') + 'What operating system is running? Cite only the attached observation.'
        self.cli = subprocess.Popen([str(self.binaries['aiosctl']), 'ask', question, '--json'], env=self.env,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        return {'cli_pid': self.cli.pid, 'uid': os.getuid()}

    def result(self):
        self.verify()
        if self.cli is None:
            raise RuntimeError('public fixture has no CLI')
        out, err = self.cli.communicate(timeout=15)
        if len(out) + len(err) > 65536:
            raise RuntimeError('public CLI response exceeded bound')
        value = {'upstream_exit': self.cli.returncode, 'answer': json.loads(out)}
        self.cli = None
        return value

    def health(self):
        self.verify()
        observed = subprocess.run([str(self.binaries['aiosctl']), 'inspect', 'service', 'sshd.service', '--json'],
                                  env=self.env, capture_output=True, check=True, timeout=5)
        service = json.loads(observed.stdout)
        if service['status'] != 'ok' or service['data']['boot_id'] != Path('/proc/sys/kernel/random/boot_id').read_text().strip():
            raise RuntimeError('deterministic SSH inspection failed during model failure')
        uid = self.bus('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'GetConnectionUnixUser', 's', 'org.kde.KWin')
        if uid.strip() != b'u 1001':
            raise RuntimeError('desktop response is not the normal tester compositor')
        response = self.bus('org.kde.KWin', '/KWin', 'org.kde.KWin', 'supportInformation')
        if not response.startswith(b's ') or len(response) < 20:
            raise RuntimeError('actual compositor did not answer its fixed read request')
        return {'service': service, 'kwin_response_bytes': len(response), 'kwin_response_sha256': hashlib.sha256(response).hexdigest(),
                'kwin_uid': 1001, 'mutation_performed': False}

    def fixture_service(self, restarting=False):
        self.verify()
        name = 'aios-service-restart-fixture.service' if restarting else 'aios-service-failure-fixture.service'
        result = subprocess.run([str(self.binaries['aiosctl']),'inspect','service',name,'--json'],env=self.env,
                                capture_output=True,timeout=10)
        if len(result.stdout)+len(result.stderr)>65536: raise RuntimeError('fixture service observation exceeds bound')
        return {'upstream_exit':result.returncode,'result':json.loads(result.stdout)}

    def fixture_scope(self):
        self.verify()
        # Two real kernel-authenticated streams, neither with supplied identity.
        def connect():
            stream=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);stream.settimeout(5)
            stream.connect(str(self.runtime/'aios/session.sock'))
            pid,uid,_=struct.unpack('3i',stream.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))
            if uid!=self.uid or pid!=int(self.show()['MainPID']):
                stream.close();raise RuntimeError('fixture stream does not belong to installed broker')
            return stream
        def call(stream,operation):
            request={'schema_version':1,'request_id':str(uuid.uuid4()),'operation':operation}
            send(stream,request);reply=receive(stream)
            if reply.get('schema_version')!=1 or reply.get('request_id')!=request['request_id'] or reply.get('operation')!='response': raise RuntimeError('uncorrelated fixture reply')
            return reply
        with connect() as original,connect() as foreign:
            resolved=call(original,{'kind':'resolve_service','unit_name':'aios-service-failure-fixture.service'})
            handle=resolved['data']['service_id']
            denied=call(foreign,{'kind':'invoke','tool_call':{'kind':'tool_call','action_id':'system.service_status','arguments':{'service_id':handle}}})
            if denied['error']['code']!='PERMISSION_DENIED' or denied['data'] is not None: raise RuntimeError('foreign stream borrowed service scope')
            missing=call(original,{'kind':'resolve_service','unit_name':'aios-no-such-service-acceptance.service'})
            if missing['error']['code']!='TARGET_NOT_FOUND' or missing['data'] is not None: raise RuntimeError('missing service reported healthy data')
            return {'foreign_stream':denied,'missing_service':missing,'mutation_performed':False}

    def close(self):
        if self.cli is not None:
            if self.cli.poll() is None:
                self.cli.kill()
            self.cli.communicate(timeout=5)
        self.cli = None
        # This fixture borrows the original product service. It owns only its
        # CLI child, and must never stop, replace or unlink the shared broker.
        self.verify()


def participant(channel, account, inference_gid):
    broker = None
    try:
        drop_ids(channel, account, inference_gid)
        broker = Broker()
        send(channel, {'ready': True, 'uid': os.getuid(), 'pid': os.getpid(), 'unit': broker.proof, 'broker_identity': broker.identity,
                       'original_unit_sha256': hashlib.sha256(broker.bytes).hexdigest(),
                       'executables': {k: {'path': str(v), 'sha256': hashlib.sha256(v.read_bytes()).hexdigest()} for k, v in broker.binaries.items()}})
        while True:
            command = receive(channel)
            if command == {'kind': 'start_crash'}:
                send(channel, broker.start(True))
            elif command == {'kind': 'start_short'}:
                send(channel, broker.start(False))
            elif command == {'kind': 'result'}:
                send(channel, broker.result())
            elif command == {'kind': 'health'}:
                send(channel, broker.health())
            elif command == {'kind': 'fixture_service'}:
                send(channel, broker.fixture_service())
            elif command == {'kind': 'fixture_restart_service'}:
                send(channel, broker.fixture_service(True))
            elif command == {'kind': 'fixture_scope'}:
                send(channel, broker.fixture_scope())
            elif command == {'kind': 'stop'}:
                broker.close(); broker = None
                send(channel, {'closed': True})
                break
            else:
                raise ValueError('unknown fixed public fixture operation')
    except (OSError, RuntimeError, ValueError, KeyError, subprocess.SubprocessError) as error:
        try:
            send(channel, {'failure': type(error).__name__, 'detail': str(error)[:300]})
        except OSError:
            pass
    finally:
        try:
            if broker is not None:
                broker.close()
        finally:
            channel.close()
            # A failed normal-user cleanup must never unwind into the root
            # coordinator's inherited stack. Missing acknowledgement fails it.
            os._exit(0)


class PublicProbe:
    def __init__(self, expected, recheck):
        if os.getuid() != 0 or os.geteuid() != 0:
            raise PermissionError('public coordinator requires initial root fixture')
        self.expected, self.recheck = expected, recheck
        recheck(expected)
        account = pwd.getpwnam('tester')
        if account.pw_uid != 1001:
            raise RuntimeError('fixed public tester identity mismatch')
        group = grp.getgrnam('aios-inference').gr_gid
        parent, child = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        parent.settimeout(20)
        # The child waits between fixed commands while root measures restart
        # backoff, recovery and immutable artifact hashes. The unit's 180-second
        # RuntimeMaxSec bounds its entire lifetime; individual calls stay at 20.
        child.settimeout(180)
        pid = os.fork()
        if pid == 0:
            parent.close()
            participant(child, account, group)
        child.close()
        self.peer = {'channel': parent, 'pid': pid, 'uid': 1001, 'gid': account.pw_gid, 'inference_gid': group}
        try:
            self.proof = receive(parent)
            if self.proof.get('ready') is not True or self.proof.get('uid') != 1001 or self.proof.get('pid') != pid:
                raise RuntimeError('public client failed before ready: ' + json.dumps(self.proof))
            self.proof['process'] = attest(self.peer)
            recheck(expected)
        except Exception:
            cleanup([self.peer])
            raise

    def call(self, kind):
        self.recheck(self.expected); attest(self.peer)
        send(self.peer['channel'], {'kind': kind})
        value = receive(self.peer['channel'])
        attest(self.peer); self.recheck(self.expected)
        if 'failure' in value:
            raise RuntimeError('fixed public client failed: ' + json.dumps(value))
        return value

    def close(self):
        try:
            if self.call('stop') != {'closed': True}:
                raise RuntimeError('public fixture did not confirm owned-unit cleanup')
        finally:
            cleanup([self.peer])


def require_failure(value, code):
    answer = value['answer']
    if (value['upstream_exit'] != 1 or answer['state'] != 'failed' or answer['error'] != code
            or answer['output'] is not None or answer['mutation_performed']):
        raise RuntimeError('public request did not fail cleanly with ' + code)


def require_system_answer(value, boot_id):
    """Verify recovery from native evidence, independent of brand capitalization."""
    answer = value['answer']
    if (value['upstream_exit'] != 0 or answer['state'] != 'completed' or answer['error'] is not None
            or answer['mutation_performed'] is not False):
        raise RuntimeError('public model recovery did not complete without effects')
    output = answer['output']
    response = output['response']
    observations = output['evidence']
    if (output['local_cpu'] is not True or output['mutation_performed'] is not False
            or response['kind'] != 'answer' or not re.search(r'\bnixos\b', response['text'].casefold())
            or not observations):
        raise RuntimeError('public recovery lacks a native CPU system answer')
    ids = set()
    for observation in observations:
        if (observation['complete'] is not True or observation['error'] is not None
                or observation['source']['provider'] != 'aios-system' or observation['data']['os_id'] != 'nixos'
                or observation['data']['boot_id'] != boot_id):
            raise RuntimeError('public recovery evidence is incomplete or from another boot')
        ids.update(observation['evidence_ids'])
    cited = response['evidence_ids']
    if not cited or not set(cited).issubset(ids):
        raise RuntimeError('public recovery citations do not reference its fresh native evidence')
