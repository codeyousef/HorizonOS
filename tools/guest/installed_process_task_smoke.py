#!/usr/bin/env python3
"""Actual installed broker and CPU-model selected-process task qualification.

No replacement daemon, claimed caller, arbitrary command, or signal operation.
The foreground SSH subject remains alive for all original task reads.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import socket
import struct
import time
import uuid

import snapshot
from model_public_fixture import Broker
from model_service_smoke import Client


def request(operation):
    return {'schema_version': 1, 'request_id': str(uuid.uuid4()), 'operation': operation}


def invoke(action, arguments):
    return {'kind': 'invoke', 'tool_call': {'kind': 'tool_call', 'action_id': action, 'arguments': arguments}}


def submit(handle):
    return {'kind': 'submit', 'request': {'mode': 'ask', 'text':
            'Inspect the explicitly selected process. Report its PID using only fresh process evidence and cite it.',
            'client_nonce': str(uuid.uuid4()), 'context_handles': [handle]}}


def require_answer(value, identity, handle):
    if (value.get('state') != 'completed' or value.get('error') is not None
            or value.get('mutation_performed') is not False):
        raise RuntimeError('installed selected-process task failed: ' + json.dumps(value))
    output = value['output']
    if output.get('local_cpu') is not True or output.get('mutation_performed') is not False:
        raise RuntimeError('task has no actual local CPU completion')
    answer = output['response']
    if answer.get('kind') != 'answer' or not isinstance(answer.get('text'), str) or not answer['text'].strip():
        raise RuntimeError('process task did not answer')
    cited = answer['evidence_ids']
    matching = [item for item in output['evidence'] if item.get('complete') is True and item.get('error') is None
                and item.get('data', {}).get('process_id') == handle
                and all(item['data'].get(k) == identity[k] for k in ('pid', 'start_time_ticks', 'executable_identity'))
                and item.get('source', {}).get('provider') == 'linux-own-user-processes'
                and set(item.get('evidence_ids', [])).intersection(cited)]
    if not matching:
        raise RuntimeError('answer lacks cited exact native selected-process identity')
    if not re.search(r'(?<![0-9])' + str(identity['pid']) + r'(?![0-9])', answer['text']):
        raise RuntimeError('answer did not report the independently observed PID')


class Unix:
    def __init__(self, broker):
        broker.verify()
        self.client = Client(broker.runtime / 'aios/session.sock')
        pid, uid, _ = struct.unpack('3i', self.client.socket.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        if (pid, uid) != (broker.identity['pid'], broker.uid):
            self.client.socket.close()
            raise RuntimeError('Unix task client is not connected to installed broker')

    def call(self, operation):
        reply = self.client.call(operation)
        if reply['error'] is not None:
            raise RuntimeError('installed operation refused: ' + json.dumps(reply))
        return reply['data']

    def refuse(self, operation, code):
        reply = self.client.call(operation)
        if reply.get('data') is not None or reply.get('error', {}).get('code') != code:
            raise RuntimeError('original task scope refusal failed: ' + json.dumps(reply))
        return reply

    def close(self):
        self.client.socket.close()


def wait(call, task):
    deadline = time.monotonic() + 95
    while time.monotonic() < deadline:
        value = call({'kind': 'get_status', 'task_id': task})
        if value['state'] in ('completed', 'failed', 'cancelled'):
            return value
        time.sleep(0.15)
    raise RuntimeError('selected-process task did not terminate within its budget')


def main():
    identity = snapshot.identity()
    if (os.getuid() != 1000 or os.geteuid() != 1000 or identity['guest_role'] != 'development'
            or Path('/etc/aios/model-test-profile').read_text().strip() != 'installed-normal-cpu-model-v1'):
        raise RuntimeError('requires enrolled dev subject in the actual installed CPU-model image')
    broker = Broker(uid=1000)
    clients = []
    try:
        release = Path(__file__).resolve().parents[2]
        # Use a separate system manager command; never user-supplied paths.
        import subprocess
        unit = subprocess.check_output(['/run/current-system/sw/bin/systemctl', '--system', 'show',
                                        'aios-model.service', '--property=ExecStart'], timeout=5).decode()
        match = re.search(r'--model-directory (/nix/store/[a-z0-9]{32}-horizon-os-model-normal-[a-f0-9]{12})', unit)
        if not match or '--qualification' in unit:
            raise RuntimeError('model is not the installed normal service')
        lock = (Path(match[1]) / 'lock.json').read_bytes()
        if lock != (release / 'models/lock.json').read_bytes():
            raise RuntimeError('installed model lock differs from frozen qualification source')
        uid_fields = next(line.split()[1:] for line in Path('/proc/self/status').read_text().splitlines() if line.startswith('Uid:'))
        if uid_fields != [str(os.getuid())] * 4:
            raise RuntimeError('native process has mixed credentials')
        native = {'pid': os.getpid(), 'uid': os.getuid(), 'boot_id': identity['boot_id'],
                  'start_time_ticks': int(Path('/proc/self/stat').read_text().rsplit(') ', 1)[1].split()[19])}
        exe = Path('/proc/self/exe').stat()
        native['executable_identity'] = (f'dev={exe.st_dev};ino={exe.st_ino};size={exe.st_size};'
            f'mtime={exe.st_mtime_ns // 10**9}:{exe.st_mtime_ns % 10**9};'
            f'ctime={exe.st_ctime_ns // 10**9}:{exe.st_ctime_ns % 10**9}')
        original, foreign = Unix(broker), Unix(broker)
        clients.extend([original, foreign])
        selection_started = time.monotonic()
        rows = original.call(invoke('process.list', {'limit': 100}))['data']['processes']
        selected = next(item for item in rows if item['pid'] == os.getpid())
        handle = selected['process_id']
        observed = original.call(invoke('process.inspect', {'process_id': handle}))
        if not all(observed['data'].get(k) == native[k] for k in ('pid', 'start_time_ticks', 'executable_identity')):
            raise RuntimeError('public selection does not match independent native identity')
        refusals = {'foreign_handle': foreign.refuse(submit(handle), 'PERMISSION_DENIED'),
                    'unknown_handle': original.refuse(submit(str(uuid.uuid4())), 'TARGET_NOT_FOUND')}
        task = original.call(submit(handle))['request_id']
        refusals['foreign_task'] = foreign.refuse({'kind': 'get_status', 'task_id': task}, 'PERMISSION_DENIED')
        unix_answer = wait(original.call, task)
        print('AIOS_PROCESS_TASK_PARTIAL=' + json.dumps({'phase': 'model_answer', 'native_identity': native,
              'selection': observed, 'unix_answer': unix_answer}, sort_keys=True), flush=True)
        require_answer(unix_answer, native, handle)
        original.call({'kind': 'forget', 'task_id': task})
        refusals['forgotten_task'] = original.refuse({'kind': 'get_status', 'task_id': task}, 'TARGET_NOT_FOUND')

        # The broker closes idle streams after ten seconds; preserve the exact
        # original client while waiting for suspend-inclusive handle expiry.
        deadline = selection_started + 30.1
        while time.monotonic() < deadline:
            original.call({'kind': 'get_capabilities'})
            time.sleep(0.15)
        refusals['expired_handle'] = original.refuse(submit(handle), 'TARGET_NOT_FOUND')
        broker.verify()
        if snapshot.identity() != identity:
            raise RuntimeError('target changed during selected-process qualification')
        proof = {'evidence_kind': 'real-installed-process-task', 'uid': broker.uid,
                 'boot_id': identity['boot_id'], 'broker_identity': broker.identity,
                 'installed_executable': str(broker.binaries['aios-sessiond']),
                 'model_lock_sha256': hashlib.sha256(lock).hexdigest(),
                 'model_sha256': json.loads(lock)['artifact']['sha256'], 'native_identity': native,
                 'selection': observed, 'unix_answer': unix_answer, 'refusals': refusals,
                 'original_unix_task_verified': True, 'native_citation_verified': True,
                 'foreign_handle_refused': True, 'foreign_task_refused': True,
                 'unknown_handle_refused': True, 'forgotten_task_refused': True,
                 'expired_handle_refused': True, 'termination_performed': False}
        print('AIOS_INSTALLED_PROCESS_TASK=' + json.dumps(proof, sort_keys=True), flush=True)
    finally:
        for client in clients:
            client.close()
        broker.close()


if __name__ == '__main__':
    main()
