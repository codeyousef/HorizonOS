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
    def __init__(self):
        if os.getuid() != 1001 or os.geteuid() != 1001:
            raise PermissionError('public fixture must first drop to tester')
        self.runtime = Path('/run/user/1001')
        info = self.runtime.lstat()
        if (self.runtime.resolve() != self.runtime or not stat.S_ISDIR(info.st_mode)
                or info.st_uid != 1001 or info.st_mode & 0o077):
            raise RuntimeError('public fixture runtime identity failed')
        self.env = {**os.environ, 'HOME': '/home/tester', 'PATH': '/run/current-system/sw/bin',
                    'XDG_RUNTIME_DIR': str(self.runtime), 'DBUS_SESSION_BUS_ADDRESS': 'unix:path=' + str(self.runtime / 'bus')}
        self.unit = 'aios-model-failure-' + uuid.uuid4().hex + '.service'
        self.binaries = {}
        for name, package in (('aios-sessiond', 'aios-core'), ('aiosctl', 'aios-cli')):
            binary = Path('/run/current-system/sw/bin', name).resolve(strict=True)
            info = binary.stat()
            if (not re.fullmatch(r'/nix/store/[a-z0-9]{32}-' + package + r'-[A-Za-z0-9._+-]+/bin/' + name, str(binary))
                    or not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o222 or not info.st_mode & 0o111):
                raise RuntimeError('public fixture requires exact protected installed binaries')
            self.binaries[name] = binary
        self.bytes = (self.binaries['aios-sessiond'].parents[1] / 'share/systemd/user/aios-sessiond.service').read_bytes()
        if not 0 < len(self.bytes) <= 65536 or self.ctl('show', self.unit, '--property=LoadState').stdout.strip() != b'LoadState=not-found':
            raise RuntimeError('public fixture must not replace a unit')
        if self.bus('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'NameHasOwner', 's', 'org.aios.Session1').strip() != b'b false':
            raise RuntimeError('public fixture must not replace a broker owner')
        self.path = self.runtime / 'systemd/user' / self.unit
        self.path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        if self.path.parent.resolve() != self.path.parent or self.path.parent.stat().st_uid != 1001:
            raise RuntimeError('unsafe public fixture unit directory')
        fd = os.open(self.path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, 'wb') as output:
            output.write(self.bytes)
        self.cli = None
        self.ctl('daemon-reload')
        self.ctl('start', self.unit)
        self.proof = self.show()
        expected = {'NoNewPrivileges': 'yes', 'PrivateNetwork': 'yes', 'ProtectHome': 'tmpfs', 'ProtectSystem': 'strict',
                    'MemoryMax': '268435456', 'TasksMax': '64', 'RuntimeDirectoryMode': '0700', 'ActiveState': 'active'}
        if (any(self.proof[k] != v for k, v in expected.items()) or self.proof['FragmentPath'] != str(self.path)
                or not self.owns() or int(self.proof['MainPID']) <= 1):
            raise RuntimeError('original installed broker hardening failed')

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
                 'ProtectSystem', 'MemoryMax', 'TasksMax', 'RuntimeDirectoryMode')
        value = self.ctl('show', self.unit, *['--property=' + name for name in names]).stdout.decode()
        return dict(line.split('=', 1) for line in value.splitlines() if '=' in line)

    def owns(self):
        info = self.path.lstat()
        return (stat.S_ISREG(info.st_mode) and info.st_uid == 1001 and info.st_nlink == 1
                and stat.S_IMODE(info.st_mode) == 0o600 and self.path.read_bytes() == self.bytes)

    def start(self, long):
        if self.cli is not None:
            raise RuntimeError('public fixture already has a CLI')
        question = ('word ' * 4500 if long else '') + 'What operating system is running? Cite only the attached observation.'
        self.cli = subprocess.Popen([str(self.binaries['aiosctl']), 'ask', question, '--json'], env=self.env,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        return {'cli_pid': self.cli.pid, 'uid': os.getuid()}

    def result(self):
        if self.cli is None:
            raise RuntimeError('public fixture has no CLI')
        out, err = self.cli.communicate(timeout=15)
        if len(out) + len(err) > 65536:
            raise RuntimeError('public CLI response exceeded bound')
        value = {'upstream_exit': self.cli.returncode, 'answer': json.loads(out)}
        self.cli = None
        return value

    def health(self):
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
        name = 'aios-service-restart-fixture.service' if restarting else 'aios-service-failure-fixture.service'
        result = subprocess.run([str(self.binaries['aiosctl']),'inspect','service',name,'--json'],env=self.env,
                                capture_output=True,timeout=10)
        if len(result.stdout)+len(result.stderr)>65536: raise RuntimeError('fixture service observation exceeds bound')
        return {'upstream_exit':result.returncode,'result':json.loads(result.stdout)}

    def fixture_scope(self):
        # Two real kernel-authenticated streams, neither with supplied identity.
        def connect():
            stream=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);stream.settimeout(5)
            stream.connect(str(self.runtime/'aios/session.sock'))
            pid,uid,_=struct.unpack('3i',stream.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))
            if uid!=1001 or pid!=int(self.show()['MainPID']):
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
        if not self.owns() or self.show()['FragmentPath'] != str(self.path):
            raise RuntimeError('public fixture unit changed; cleanup refused')
        self.ctl('stop', self.unit)
        if self.show()['MainPID'] != '0':
            raise RuntimeError('owned broker did not stop')
        self.path.unlink()
        self.ctl('daemon-reload')


def participant(channel, account, inference_gid):
    broker = None
    try:
        drop_ids(channel, account, inference_gid)
        broker = Broker()
        send(channel, {'ready': True, 'uid': os.getuid(), 'pid': os.getpid(), 'unit': broker.proof,
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
