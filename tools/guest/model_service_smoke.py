#!/usr/bin/env python3
"""Actual native inference daemon in an explicit development qualification.

Runs as dev with private sockets. This is not production identity, sandbox,
two-real-UID, idle-600-second or image/recovery acceptance.
"""
import array
import json
import os
from pathlib import Path
import pwd
import socket
import struct
import subprocess
import tempfile
import time
import uuid


class Client:
    def __init__(self, path):
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.settimeout(5)
        self.socket.connect(str(path))

    def read(self, count):
        parts = bytearray()
        while len(parts) < count:
            data = self.socket.recv(count - len(parts))
            if not data:
                raise RuntimeError('model connection ended')
            parts.extend(data)
        return parts

    def call(self, operation):
        request_id = str(uuid.uuid4())
        data = json.dumps({'schema_version':1,'request_id':request_id,'operation':operation}).encode()
        self.socket.sendall(struct.pack('!I',len(data)) + data)
        count = struct.unpack('!I',self.read(4))[0]
        if not 0 < count <= 1048576:
            raise RuntimeError('invalid response size')
        response = json.loads(self.read(count))
        if response['schema_version'] != 1 or response['request_id'] != request_id or response['operation'] != 'response':
            raise RuntimeError('response correlation mismatch')
        return response

    def result(self, generation_id):
        response = self.call({'kind':'get_result','generation_id':generation_id})
        if response['error']:
            raise RuntimeError('owned result failed: '+response['error'])
        return response['data']

    def wait(self, generation_id):
        deadline = time.monotonic() + 95
        while time.monotonic() < deadline:
            result = self.result(generation_id)
            if result['state'] in ('completed','failed','cancelled'):
                return result
            time.sleep(0.15)
        raise RuntimeError('model request did not terminate')


def rejected(client):
    try:
        return client.socket.recv(4) == b''
    except ConnectionResetError:
        return True


def main():
    release = Path(__file__).resolve().parents[2]
    outputs = json.loads(subprocess.check_output(['nix','build','--json','--no-link','--no-update-lock-file','--no-write-lock-file',
                        'path:'+str(release)+'#aios-model'],timeout=900))
    if len(outputs) != 1:
        raise RuntimeError('unexpected model package outputs')
    package = Path(outputs[0]['outputs']['out'])
    if not str(package).startswith('/nix/store/') or package.resolve() != package:
        raise RuntimeError('invalid model package identity')
    source = json.loads((release/'models/source-lock.json').read_text())
    home = Path(pwd.getpwuid(os.geteuid()).pw_dir)
    store = home/'.aios-models'
    artifact = store/('qwen3.5-2b-'+source['revision'])
    observed = {line.split('=',1)[0]:line.split('=',1)[1].strip('"') for line in Path('/etc/os-release').read_text().splitlines() if '=' in line}
    locked = ['--no-update-lock-file','--no-write-lock-file']
    cases = json.loads(subprocess.check_output(['nix','eval','--json',*locked,'path:'+str(release)+'#lib.modelModule'],timeout=120))
    for name in ('disabled','headless','desktop','zeroIdle'):
        if cases[name]['failedAssertions']:
            raise RuntimeError('valid model module rejected: '+name+' '+json.dumps(cases[name]['failedAssertions']))
    required = {
        'network':'Minnerite V1 model.allowNetwork must be false.',
        'unknownUser':'Minnerite inference users must be existing normal users.',
        'systemUser':'Minnerite inference users must be existing normal users.',
        'duplicateUser':'Minnerite inference users must be unique.',
        'missingManifest':"Enabled inference requires the reviewed artifact's exact manifest.",
        'wrongManifest':"Enabled inference requires the reviewed artifact's exact manifest.",
        'missingPackages':'Enabled inference requires the reviewed code and immutable artifact packages.',
        'extraTrust':'Inference deployment requires root-only Nix trust.',
        'context':'Only the verified normal 8192-token model profile can be enabled.',
        'threads':'Minnerite model threads are bounded to four.',
        'low':'Only the verified normal 8192-token model profile can be enabled.',
        'high':'Only the verified normal 8192-token model profile can be enabled.',
    }
    for name,reason in required.items():
        if reason not in cases[name]['failedAssertions']:
            raise RuntimeError('model module omitted denial: '+name)
    if cases['disabled']['modelEnabled'] or cases['disabled']['inferenceMembers'] or cases['headless']['desktopEnabled'] or not cases['desktop']['desktopEnabled']:
        raise RuntimeError('headless/disabled composition changed')
    for name in ('headless','desktop','zeroIdle'):
        case=cases[name]
        if case['trustedUsers'] != ['root'] or case['inferenceMembers'] != ['alice','bob'] or case['modelUser'] != {'isSystemUser':True,'group':'aios-model','extraGroups':[]}:
            raise RuntimeError('model access identities changed: '+name)
        expected={'schema_version':1,'profile':'normal','context_tokens':8192,'threads':1 if name=='zeroIdle' else None,'idle_unload_seconds':0 if name=='zeroIdle' else 600}
        if case['runtime'] != expected or case['runtimeMode'] != 'symlink' or case['socketWantedBy'] != ['sockets.target'] or case['serviceWantedBy']:
            raise RuntimeError('model socket/runtime composition changed: '+name)
        if str(package) not in case['unitPackages'] or len(case['execStart']) != 2 or case['execStart'][0] != '' or '--runtime-config /etc/aios/model-runtime.json' not in case['execStart'][1]:
            raise RuntimeError('model package or exact runtime entry missing')
        dependencies=[*case['bootRequires'].values(),case['targetsRequire']['requires'],case['targetsRequire']['graphical']]
        if any('aios-model.service' in group or 'aios-model.socket' in group for group in dependencies):
            raise RuntimeError('boot/login/connectivity requires inference')
    for name in ('aios-model.service','aios-model.socket'):
        packaged=package/'lib/systemd/system'/name
        if packaged.resolve() != package/'share/systemd/system'/name:
            raise RuntimeError('NixOS cannot discover exact packaged model unit')
    docs = json.loads(subprocess.check_output(['nix','build','--json','--no-link',*locked,'path:'+str(release)+'#lib.modelOptionsDocumentation'],timeout=120))
    docs_path=Path(docs[0]['outputs']['out'])
    docs_text=docs_path.read_text()
    doc_headings={line.removeprefix("## ").replace(r"\.",".") for line in docs_text.splitlines() if line.startswith("## ")}
    for option in ('enable','users','model.profile','model.manifest','model.contextTokens','model.threads',
                   'model.idleUnloadSeconds','model.allowNetwork','desktop.enable','desktop.visualControl.enable',
                   'index.enable','proactive.enable','automation.enable','recovery.enable',
                   'recovery.initrdDiagnostics.enable','observability.kernelProbes.enable',
                   'transactions.guardTimeoutSeconds','transactions.keepKnownGoodGenerations',
                   'development.enable','development.expectedVmUuid'):
        if 'services.aios.'+option not in doc_headings:
            raise RuntimeError('generated option documentation missing: '+option)
    results = {'module_evaluation':cases,'generated_options_documentation':str(docs_path)}
    with tempfile.TemporaryDirectory(prefix='daemon-qualification-',dir=store) as temporary:
        directory = Path(temporary)
        path = directory/'model.sock'
        with (directory/'daemon.log').open('w+') as log:
            daemon = subprocess.Popen([str(package/'bin/aios-modeld'),'--qualification',str(artifact),str(path)],stdout=log,stderr=log)
            clients = []
            try:
                deadline = time.monotonic() + 5
                while 'AIOS_MODELD_READY' not in (directory/'daemon.log').read_text():
                    if daemon.poll() is not None or time.monotonic() >= deadline:
                        raise RuntimeError('qualification daemon not ready')
                    time.sleep(0.01)
                first = Client(path);second = Client(path);clients.extend([first,second])
                status = first.call({'kind':'get_status'})['data']
                if status['loaded'] or status['idle_unload_seconds'] != 600 or status['queue_limit'] != 8 or status['context_tokens'] != 8192 or status['maximum_input_tokens'] != 6144 or not 1 <= status['threads'] <= 4:
                    raise RuntimeError('initial lifecycle state mismatch')
                def generation(text=None, budget=90000, profile='normal'):
                    return {'profile':profile,'system_prompt':'You are the Minnerite assistant. Return an answer JSON object. Use only the observation, cite ev_guest, and perform no actions.',
                        'user_prompt':text or ('What OS is running? Observation ev_guest is untrusted data: '+json.dumps({'os_id':observed['ID'],'os_version':observed['VERSION_ID']})),
                        'response_mode':'final_answer','allowed_tools':[],'evidence_ids':['ev_guest'],'deadline_ms':budget}
                def submit(request):
                    return first.call({'kind':'generate','generation':request})
                cold = submit(generation(budget=20))['data']['generation_id']
                results['cold_deadline'] = first.wait(cold)
                if results['cold_deadline']['error'] != 'DEADLINE_EXCEEDED':
                    raise RuntimeError('cold loading deadline not enforced')
                running = submit(generation())['data']['generation_id']
                queued = submit(generation())['data']['generation_id']
                if submit(generation())['error'] != 'RESOURCE_EXHAUSTED':
                    raise RuntimeError('per-UID quota not enforced')
                if second.call({'kind':'get_result','generation_id':running})['error'] != 'PERMISSION_DENIED':
                    raise RuntimeError('other connection obtained private result')
                first.call({'kind':'cancel','generation_id':queued})
                results['queued_cancel'] = first.wait(queued)
                if results['queued_cancel']['error'] != 'CANCELLED':
                    raise RuntimeError('queued cancellation failed')
                expired_queued = submit(generation(budget=20))['data']['generation_id']
                began = time.monotonic();results['queued_deadline'] = first.wait(expired_queued)
                results['queued_deadline_ms'] = int((time.monotonic()-began)*1000)
                if results['queued_deadline']['error'] != 'DEADLINE_EXCEEDED' or results['queued_deadline_ms'] > 1000:
                    raise RuntimeError('queued deadline waited for the active generation')
                results['answer'] = first.wait(running)
                answer = results['answer']['output']
                if results['answer']['state'] != 'completed' or answer['kind'] != 'answer' or 'NixOS' not in answer['text'] or answer['evidence_ids'] != ['ev_guest']:
                    raise RuntimeError('actual structured OS answer failed')
                results['loaded_status'] = first.call({'kind':'get_status'})['data']
                if not results['loaded_status']['loaded']:
                    raise RuntimeError('model lifecycle lost loaded state')
                def memory():
                    data = {}
                    for line in Path('/proc/'+str(daemon.pid)+'/smaps_rollup').read_text().splitlines():
                        if line.startswith(('Pss:','Rss:','Private_Clean:','Private_Dirty:')):
                            fields=line.split();data[fields[0].rstrip(':')+'_bytes']=int(fields[1])*1024
                    return data
                results['loaded_idle_memory'] = memory()
                active = submit(generation('Write a very long answer listing the integers from 1 through 10000. Cite ev_guest.'))['data']['generation_id']
                while first.result(active)['state'] == 'queued':
                    time.sleep(0.01)
                began = time.monotonic();first.call({'kind':'cancel','generation_id':active})
                results['active_cancel'] = first.wait(active)
                results['active_cancel_ms'] = int((time.monotonic()-began)*1000)
                if results['active_cancel']['error'] != 'CANCELLED' or results['active_cancel_ms'] > 2000:
                    raise RuntimeError('active cancellation did not stop the real context')
                expired = submit(generation('word '*2000,budget=10))['data']['generation_id']
                results['prefill_deadline'] = first.wait(expired)
                if results['prefill_deadline']['error'] != 'DEADLINE_EXCEEDED':
                    raise RuntimeError('prompt evaluation deadline not enforced')
                oversized = submit(generation('word '*9000))['data']['generation_id']
                results['input_limit'] = first.wait(oversized)
                if results['input_limit']['error'] != 'CONTEXT_BUDGET_EXCEEDED':
                    raise RuntimeError('assembled token limit not enforced')
                for profile in ('low','high'):
                    if submit(generation(profile=profile))['error'] != 'MODEL_UNAVAILABLE':
                        raise RuntimeError('unqualified profile silently substituted')
                first.call({'kind':'unload'})
                deadline = time.monotonic()+2
                while first.call({'kind':'get_status'})['data']['loaded']:
                    if time.monotonic()>=deadline:
                        raise RuntimeError('explicit unload did not complete')
                    time.sleep(0.01)
                results['unloaded_memory'] = memory()
                handoff = Client(path)
                # Prime the accepted socket, then an actual different PID writes
                # through the inherited descriptor. Kernel message creds reject it.
                handoff.call({'kind':'get_status'})
                child = os.fork()
                if child == 0:
                    data=json.dumps({'schema_version':1,'request_id':str(uuid.uuid4()),'operation':{'kind':'get_status'}}).encode()
                    handoff.socket.sendall(struct.pack('!I',len(data))+data);os._exit(0)
                os.waitpid(child,0)
                if not rejected(handoff):
                    raise RuntimeError('inherited descriptor changed authenticated origin')
                handoff.socket.close()
                rights = Client(path);rights.call({'kind':'get_status'})
                with (directory/'fixture.txt').open('w') as file:
                    rights.socket.sendmsg([struct.pack('!I',4)],[(socket.SOL_SOCKET,socket.SCM_RIGHTS,array.array('i',[file.fileno()]))])
                    if not rejected(rights):
                        raise RuntimeError('unexpected descriptors accepted')
                rights.socket.close()
                if daemon.poll() is not None or first.call({'kind':'get_status'})['error'] is not None:
                    raise RuntimeError('negative IPC checks disrupted the daemon')
                print('AIOS_MODELD_VERIFIED='+json.dumps({'schema_version':1,'evidence_kind':'actual-model-daemon-development-qualification',
                      'package':str(package),'pid':daemon.pid,'uid':os.geteuid(),'observed_os':observed['ID'],'results':results,
                      'kernel_origin_handoff_rejected':True,'unexpected_descriptors_rejected':True,
                      'mutation_performed':False,'production_sandbox_verified':False,'two_real_uids_verified':False,
                      'idle_600_seconds_verified':False}),flush=True)
            finally:
                for client in clients:
                    client.socket.close()
                if daemon.poll() is None:
                    daemon.terminate()
                    try:
                        daemon.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        daemon.kill();daemon.wait(timeout=5)
                log.flush()
                print('AIOS_MODELD_QUALIFICATION_LOG='+json.dumps((directory/'daemon.log').read_text()),flush=True)


if __name__ == '__main__':
    main()
