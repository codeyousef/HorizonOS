"""Fixed five-normal-UID load probe for the disposable model acceptance image.

The root coordinator drops every child's IDs before connecting to inference.
Private coordination uses bounded JSON, never unprivileged pickle. No CLI/RPC.
"""
import grp
import json
import os
from pathlib import Path
import pwd
import signal
import socket
import struct
import time

from model_service_smoke import Client
import snapshot

SUBJECTS = (('dev', 1000), ('tester', 1001), ('model-load-a', 1100),
            ('model-load-b', 1101), ('model-load-c', 1102))
MAX_FRAME = 32768


def send(channel, value):
    data = json.dumps(value, allow_nan=False, separators=(',', ':')).encode()
    if not 0 < len(data) <= MAX_FRAME:
        raise ValueError('private queue frame exceeds bound')
    channel.sendall(struct.pack('!I', len(data)) + data)


def receive(channel):
    def exact(count):
        data = bytearray()
        while len(data) < count:
            part = channel.recv(count - len(data))
            if not part:
                raise RuntimeError('private queue coordinator ended')
            data.extend(part)
        return bytes(data)
    count = struct.unpack('!I', exact(4))[0]
    if not 0 < count <= MAX_FRAME:
        raise ValueError('private queue frame exceeds bound')
    value = snapshot.decode(exact(count))
    if not isinstance(value, dict):
        raise ValueError('private queue frame is not an object')
    return value


def generation(long=False):
    # Public synthetic prefill holds the real CPU worker while peers enqueue;
    # no paused daemon, alternate engine, extra output budget or barrier hook.
    prompt = ('word ' * 5000 if long else '') + 'Observation ev_queue: this is a disposable NixOS queue test. Identify the OS.'
    return {'kind': 'generate', 'generation': {
        'profile': 'normal', 'system_prompt': 'Return an answer JSON object citing only ev_queue. Perform no actions.',
        'user_prompt': prompt, 'response_mode': 'final_answer', 'allowed_tools': [],
        'evidence_ids': ['ev_queue'], 'deadline_ms': 90000}}


def child(channel, account, inference_gid):
    client = None
    try:
        # Close inherited model/other coordinator descriptors before changing
        # identity. Only this fork's private socket survives.
        keep = channel.fileno()
        for name in os.listdir('/proc/self/fd'):
            fd = int(name)
            if fd >= 3 and fd != keep:
                try:
                    os.close(fd)
                except OSError:
                    pass
        os.setgroups(sorted({account.pw_gid, inference_gid}))
        os.setresgid(account.pw_gid, account.pw_gid, account.pw_gid)
        os.setresuid(account.pw_uid, account.pw_uid, account.pw_uid)
        if os.getresuid() != (account.pw_uid,) * 3 or os.getresgid() != (account.pw_gid,) * 3:
            raise RuntimeError('queue subject did not permanently drop root IDs')
        client = Client('/run/aios/model.sock')
        owned = []
        send(channel, {'ready': True, 'uid': os.getuid(), 'pid': os.getpid()})
        while True:
            command = receive(channel)
            kind = command.get('kind')
            if command == {'kind': 'submit', 'long': True} or command == {'kind': 'submit', 'long': False}:
                result = client.call(generation(command['long']))
                if result['error'] is None:
                    owned.append(result['data']['generation_id'])
                send(channel, result)
            elif command == {'kind': 'snapshot'}:
                send(channel, {'status': client.call({'kind': 'get_status'}),
                               'results': {item: client.result(item) for item in owned}})
            elif kind == 'foreign' and set(command) == {'kind', 'generation_id'} and isinstance(command['generation_id'], str):
                send(channel, {op: client.call({'kind': op, 'generation_id': command['generation_id']})
                               for op in ('get_result', 'cancel')})
            elif command == {'kind': 'cancel'}:
                began = time.monotonic()
                acknowledgements = [client.call({'kind': 'cancel', 'generation_id': item}) for item in owned]
                results = [client.wait(item) for item in owned]
                send(channel, {'acknowledgements': acknowledgements, 'results': results,
                               'elapsed_ms': int((time.monotonic() - began) * 1000)})
            elif command == {'kind': 'stop'}:
                break
            else:
                raise ValueError('unknown fixed queue fixture operation')
    except (OSError, RuntimeError, ValueError, KeyError) as error:
        try:
            send(channel, {'failure': type(error).__name__})
        except OSError:
            pass
    finally:
        if client is not None:
            client.socket.close()
        channel.close()
    os._exit(0)


def attest(peer):
    directory = Path('/proc') / str(peer['pid'])
    value = dict(line.split(':', 1) for line in (directory / 'status').read_text().splitlines() if ':' in line)
    ticks = (directory / 'stat').read_text().split(') ', 1)[1].split()[19]
    if (value['Uid'].split() != [str(peer['uid'])] * 4
            or value['Gid'].split() != [str(peer['gid'])] * 4
            or set(map(int, value['Groups'].split())) != {peer['gid'], peer['inference_gid']}
            or any(int(value[key], 16) for key in ('CapEff', 'CapPrm', 'CapInh', 'CapAmb'))
            or value['NoNewPrivs'].strip() != '1'
            or peer.get('start_ticks', ticks) != ticks):
        raise RuntimeError('actual dropped-UID queue process identity changed')
    peer['start_ticks'] = ticks
    return {'uid': peer['uid'], 'gid': peer['gid'], 'pid': peer['pid'], 'start_ticks': ticks,
            'no_new_privileges': True, 'effective_permitted_inheritable_ambient_capabilities': 0}


def cleanup(peers):
    for peer in peers:
        try:
            send(peer['channel'], {'kind': 'stop'})
        except OSError:
            pass
        peer['channel'].close()
    deadline = time.monotonic() + 2
    remaining = {peer['pid'] for peer in peers}
    while remaining and time.monotonic() < deadline:
        for pid in tuple(remaining):
            if os.waitpid(pid, os.WNOHANG)[0] == pid:
                remaining.remove(pid)
        if remaining:
            time.sleep(0.01)
    # Only unreaped children created by this coordinator can be signalled.
    for pid in remaining:
        os.kill(pid, signal.SIGKILL)
        os.waitpid(pid, 0)


def run(expected, recheck, process):
    if os.getuid() != 0 or os.geteuid() != 0:
        raise PermissionError('fixed queue coordinator requires initial root fixture')
    recheck(expected)
    peers = []
    try:
        inference_gid = grp.getgrnam('aios-inference').gr_gid
        for name, uid in SUBJECTS:
            account = pwd.getpwnam(name)
            if account.pw_uid != uid or uid == 0:
                raise RuntimeError('fixed queue subject identity mismatch')
            parent, participant = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
            parent.settimeout(6); participant.settimeout(6)
            pid = os.fork()
            if pid == 0:
                parent.close()
                child(participant, account, inference_gid)
            participant.close()
            peer = {'channel': parent, 'pid': pid, 'uid': uid, 'gid': account.pw_gid, 'inference_gid': inference_gid}
            peers.append(peer)
            if receive(parent) != {'ready': True, 'uid': uid, 'pid': pid}:
                raise RuntimeError('normal queue participant failed before ready')
            attest(peer)

        def exchange(peer, command):
            recheck(expected); attest(peer)
            send(peer['channel'], command)
            value = receive(peer['channel'])
            attest(peer); recheck(expected)
            if 'failure' in value:
                raise RuntimeError('queue participant failed: ' + value['failure'])
            return value

        before = process(expected)
        accepted = []

        def submit(peer, long=False):
            value = exchange(peer, {'kind': 'submit', 'long': long})
            if value['error'] is not None:
                raise RuntimeError('actual queue rejected an allowed request')
            accepted.append(value['data']['generation_id'])
            return value

        first = submit(peers[0], long=True)
        submit(peers[0])
        # Require an actually running request before counting eight waiters.
        deadline = time.monotonic() + 2
        while exchange(peers[0], {'kind': 'snapshot'})['results'][accepted[0]]['state'] != 'running':
            if time.monotonic() > deadline:
                raise RuntimeError('real queue prefill did not enter running state')
        uid_denial = exchange(peers[0], {'kind': 'submit', 'long': False})
        if uid_denial['error'] != 'RESOURCE_EXHAUSTED':
            raise RuntimeError('two-request UID quota was not enforced')
        for peer in peers[1:4]:
            submit(peer); submit(peer)
        submit(peers[4])
        global_denial = exchange(peers[4], {'kind': 'submit', 'long': False})
        if global_denial['error'] != 'RESOURCE_EXHAUSTED' or len(set(accepted)) != 9:
            raise RuntimeError('global eight-waiter bound was not enforced')
        foreign = exchange(peers[1], {'kind': 'foreign', 'generation_id': first['data']['generation_id']})
        if any(value['error'] != 'PERMISSION_DENIED' for value in foreign.values()):
            raise RuntimeError('foreign UID obtained queued request authority')
        observations = [exchange(peer, {'kind': 'snapshot'}) for peer in peers]
        states = [value['state'] for observed in observations for value in observed['results'].values()]
        if (states.count('running') != 1 or states.count('queued') != 8
                or [item['status']['data']['own_queued'] for item in observations] != [1, 2, 2, 2, 1]
                or any(item['status']['error'] is not None or item['status']['data']['queue_limit'] != 8 for item in observations)):
            raise RuntimeError('actual full queue did not contain exactly one running and eight waiting requests')
        full = process(expected)
        if (before['pid'], before['start_ticks']) != (full['pid'], full['start_ticks']):
            raise RuntimeError('model restarted during queue load')
        # Broadcast cancellation before receiving acknowledgements: the active
        # cancellation cannot race a serial coordinator into starting all peers.
        began = time.monotonic()
        recheck(expected)
        for peer in peers:
            attest(peer); send(peer['channel'], {'kind': 'cancel'})
        cancellations = [receive(peer['channel']) for peer in peers]
        for peer in peers:
            attest(peer)
        recheck(expected)
        elapsed = int((time.monotonic() - began) * 1000)
        if (elapsed >= 2000 or any('failure' in value or value['elapsed_ms'] >= 2000 for value in cancellations)
                or any(item['error'] is not None for value in cancellations for item in value['acknowledgements'])
                or any(item['state'] != 'cancelled' or item['error'] != 'CANCELLED'
                       for value in cancellations for item in value['results'])):
            raise RuntimeError('full-queue cancellation did not finish within two seconds')
        final = [exchange(peer, {'kind': 'snapshot'}) for peer in peers]
        if any(item['status']['data']['busy'] or item['status']['data']['own_queued'] for item in final):
            raise RuntimeError('cancelled queue retained active or pending work')
        return {'schema_version': 1, 'evidence_kind': 'actual-installed-five-normal-uid-queue-load', 'verified': True,
                'participants': [attest(peer) for peer in peers], 'accepted_generation_ids': accepted,
                'running': 1, 'queued': 8, 'observations': observations, 'per_uid_denial': uid_denial,
                'global_queue_denial': global_denial, 'foreign_denials': foreign,
                'full_queue_process_pss_sample': full, 'cancellations': cancellations, 'cancel_elapsed_ms': elapsed}
    finally:
        cleanup(peers)
