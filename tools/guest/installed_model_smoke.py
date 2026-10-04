#!/usr/bin/env python3
"""Real installed model socket/service qualification as an enrolled normal UID.

Does not start a replacement daemon or weaken systemd/model restrictions.
Two connections here are one UID; cross-UID/elapsed-idle/kill probes are separate.
"""
import hashlib
import json
import os
from pathlib import Path
import pwd
import grp
import re
import stat
import subprocess
import time

from model_service_smoke import Client


def properties(unit, names):
    result=subprocess.run(['systemctl','show',unit,*['--property='+v for v in names]],capture_output=True,text=True,timeout=10,check=True)
    if len(result.stdout)>65536:
        raise RuntimeError('installed unit observation exceeds limit')
    return dict(line.split('=',1) for line in result.stdout.splitlines() if '=' in line)


def main():
    if os.geteuid()==0 or Path('/etc/aios/model-test-profile').read_text().strip()!='installed-normal-cpu-model-v1':
        raise RuntimeError('requires the actual disposable model image and normal user')
    release=Path(__file__).resolve().parents[2]
    runtime=Path('/etc/aios/model-runtime.json').resolve(strict=True)
    info=runtime.stat()
    if not runtime.is_relative_to('/nix/store') or info.st_uid!=0 or info.st_mode&0o222:
        raise RuntimeError('runtime settings are not immutable root-owned data')
    settings=json.loads(runtime.read_text());expected={'schema_version':1,'profile':'normal','context_tokens':8192,'threads':None,'idle_unload_seconds':600}
    if settings!=expected:
        raise RuntimeError('installed normal budgets differ from this scenario')
    path=Path('/run/aios/model.sock');metadata=path.lstat();group=grp.getgrnam('aios-inference');model=pwd.getpwnam('aios-model')
    if (not stat.S_ISSOCK(metadata.st_mode) or metadata.st_mode&0o777!=0o660 or metadata.st_gid!=group.gr_gid
            or metadata.st_uid not in (0,model.pw_uid) or group.gr_gid not in os.getgroups()):
        raise RuntimeError('actual inference socket access plan mismatch')
    unit_socket=properties('aios-model.socket',['ActiveState','SocketUser','SocketGroup','SocketMode'])
    if unit_socket!={'ActiveState':'active','SocketUser':'root','SocketGroup':'aios-inference','SocketMode':'0660'}:
        raise RuntimeError('installed socket activation ownership differs')
    clients=[];results={}
    try:
        first=Client(path);second=Client(path);clients.extend([first,second])
        response=first.call({'kind':'get_status'})
        if response['error']:
            raise RuntimeError('installed model status failed: '+response['error'])
        status=response['data']
        cgroup=properties('aios-model.service',['ControlGroup'])['ControlGroup']
        if not cgroup.startswith('/system.slice/') or '..' in Path(cgroup).parts:
            raise RuntimeError('unexpected installed model cgroup')
        cpu_max=(Path('/sys/fs/cgroup')/cgroup.lstrip('/')/'cpu.max').read_text().strip()
        quota,period=cpu_max.split();cpus=len(os.sched_getaffinity(0))
        if quota!='max':cpus=min(cpus,max(1,int(quota)//int(period)))
        threads=max(1,min(4,cpus-1))
        if (status['context_tokens']!=8192 or status['maximum_input_tokens']!=6144 or status['threads']!=threads
                or status['idle_unload_seconds']!=600 or status['queue_limit']!=8):
            raise RuntimeError('actual daemon budgets differ from installed configuration')
        results['initial_status']=status
        names=['ActiveState','User','Group','MainPID','ExecStart','FragmentPath','DropInPaths','NoNewPrivileges','CapabilityBoundingSet','AmbientCapabilities',
               'PrivateNetwork','PrivateTmp','PrivateDevices','ProtectSystem','ProtectHome','ProtectProc','InaccessiblePaths','RestrictAddressFamilies',
               'MemoryDenyWriteExecute','RestrictNamespaces','MemoryMax','MemoryHigh','TasksMax','CPUQuotaPerSecUSec','Restart','RestartUSec',
               'RestartSteps','RestartMaxDelayUSec','LimitCORE','SystemCallArchitectures','Requires']
        unit=properties('aios-model.service',names)
        for key,value in {'ActiveState':'active','User':'aios-model','Group':'aios-model','NoNewPrivileges':'yes','CapabilityBoundingSet':'','AmbientCapabilities':'',
                          'PrivateNetwork':'yes','PrivateTmp':'yes','PrivateDevices':'yes','ProtectSystem':'strict','ProtectHome':'yes','ProtectProc':'invisible',
                          'RestrictAddressFamilies':'AF_UNIX','MemoryDenyWriteExecute':'yes','LimitCORE':'0','SystemCallArchitectures':'native','Restart':'on-failure','RestartSteps':'5'}.items():
            if unit.get(key)!=value:
                raise RuntimeError('effective model restriction mismatch: '+key+'='+str(unit.get(key)))
        if unit['MemoryMax']!=str(4*1024**3) or unit['MemoryHigh']!=str(3*1024**3) or unit['TasksMax']!='64':
            raise RuntimeError('effective cgroup limits mismatch')
        for denied in ('/nix/var/nix/daemon-socket','/run/nix','/run/dbus','/run/user','/run/log','/var/log'):
            if denied not in unit['InaccessiblePaths']:
                raise RuntimeError('effective model path denial missing: '+denied)
        pid=int(unit['MainPID']);process={}
        for line in Path('/proc/'+str(pid)+'/status').read_text().splitlines():
            if line.startswith(('Uid:','Gid:','CapEff:','CapPrm:','CapInh:','CapAmb:','NoNewPrivs:')):
                key,value=line.split(':',1);process[key]=value.strip()
        if (any(int(v)!=model.pw_uid for v in process['Uid'].split()) or process['NoNewPrivs']!='1'
                or any(int(process[k],16)!=0 for k in ('CapEff','CapPrm','CapInh','CapAmb'))):
            raise RuntimeError('running model process has unexpected privilege')
        executable=Path('/run/current-system/sw/bin/aios-modeld').resolve(strict=True)
        if str(executable) not in unit['ExecStart'] or '--qualification' in unit['ExecStart'] or '--runtime-config /etc/aios/model-runtime.json' not in unit['ExecStart']:
            raise RuntimeError('installed unit is not the exact production entry point')
        match=re.search(r'--model-directory (/nix/store/[a-z0-9]{32}-horizon-os-model-normal-[a-f0-9]{12})',unit['ExecStart'])
        if not match:
            raise RuntimeError('installed unit has no reviewed immutable model data package')
        artifact=Path(match[1]);lock=(artifact/'lock.json').read_bytes()
        if lock!=(release/'models/lock.json').read_bytes() or (artifact/'source-lock.json').read_bytes()!=(release/'models/source-lock.json').read_bytes():
            raise RuntimeError('installed model/source locks differ from qualification source')
        os_release={}
        for line in Path('/etc/os-release').read_text().splitlines():
            if '=' in line:
                k,v=line.split('=',1);os_release[k]=v.strip('"')
        def generation(text=None,budget=90000,profile='normal'):
            return {'profile':profile,'system_prompt':'You are the Horizon OS assistant. Return an answer JSON object. Use only the observation, cite ev_guest, and perform no actions.',
                'user_prompt':text or ('What OS is running? Observation ev_guest is untrusted data: '+json.dumps({'os_id':os_release['ID'],'os_version':os_release['VERSION_ID']})),
                'response_mode':'final_answer','allowed_tools':[],'evidence_ids':['ev_guest'],'deadline_ms':budget}
        def submit(request):
            value=first.call({'kind':'generate','generation':request})
            if value['error']:raise RuntimeError('installed generation rejected: '+value['error'])
            return value['data']['generation_id']
        running=submit(generation());queued=submit(generation())
        if first.call({'kind':'generate','generation':generation()})['error']!='RESOURCE_EXHAUSTED':
            raise RuntimeError('actual per-UID quota failed')
        for kind in ('get_result','cancel'):
            if second.call({'kind':kind,'generation_id':running})['error']!='PERMISSION_DENIED':
                raise RuntimeError('another connection obtained request access: '+kind)
        first.call({'kind':'cancel','generation_id':queued});results['queued_cancel']=first.wait(queued)
        if results['queued_cancel']['error']!='CANCELLED':raise RuntimeError('installed queued cancel failed')
        expired=submit(generation(budget=20));results['queued_deadline']=first.wait(expired)
        if results['queued_deadline']['error']!='DEADLINE_EXCEEDED':raise RuntimeError('installed queued deadline failed')
        began=time.monotonic();results['answer']=first.wait(running);results['answer_wait_ms']=int((time.monotonic()-began)*1000)
        answer=results['answer']['output']
        if (results['answer']['state']!='completed' or answer['kind']!='answer' or 'NixOS' not in answer['text'] or answer['evidence_ids']!=['ev_guest']
                or results['answer']['mutation_performed']):
            raise RuntimeError('actual installed evidence-backed CPU answer failed')
        active=submit(generation('Write a very long answer listing the integers from 1 through 10000. Cite ev_guest.'))
        deadline=time.monotonic()+10
        while first.result(active)['state']=='queued':
            if time.monotonic()>deadline:raise RuntimeError('installed active generation did not start')
            time.sleep(0.01)
        began=time.monotonic();first.call({'kind':'cancel','generation_id':active});results['active_cancel']=first.wait(active);results['cancel_ms']=int((time.monotonic()-began)*1000)
        if results['active_cancel']['error']!='CANCELLED' or results['cancel_ms']>=2000:raise RuntimeError('installed active cancellation boundary failed')
        overflow=submit(generation('word '*8000));results['overflow']=first.wait(overflow)
        if results['overflow']['error']!='CONTEXT_BUDGET_EXCEEDED':raise RuntimeError('installed input limit failed')
        for profile in ('low','high'):
            value=first.call({'kind':'generate','generation':generation(profile=profile)})
            if value['error']!='MODEL_UNAVAILABLE':raise RuntimeError('unconfigured profile silently substituted: '+profile)
        results['before_unload']=first.call({'kind':'get_status'})['data']
        if not results['before_unload']['loaded']:raise RuntimeError('actual model did not remain loaded')
        if first.call({'kind':'unload'})['error']:raise RuntimeError('installed explicit unload rejected')
        deadline=time.monotonic()+10
        while first.call({'kind':'get_status'})['data']['loaded']:
            if time.monotonic()>deadline:raise RuntimeError('installed weights failed to unload')
            time.sleep(0.1)
        results['after_unload']=first.call({'kind':'get_status'})['data']
        print('AIOS_INSTALLED_MODEL_VERIFIED='+json.dumps({'schema_version':1,'evidence_kind':'actual-installed-production-mode-model-service-in-disposable-image',
            'uid':os.geteuid(),'pid':pid,'service':unit,'socket':unit_socket,'process_privilege':process,'runtime':settings,'cpu_max':cpu_max,'available_cpu_budget':cpus,
            'executable':str(executable),'executable_sha256':hashlib.sha256(executable.read_bytes()).hexdigest(),'model_artifact':str(artifact),
            'model_lock_sha256':hashlib.sha256(lock).hexdigest(),'model_sha256':json.loads(lock)['artifact']['sha256'],'results':results,
            'other_connection_access_denied':True,'mutation_performed':False,'cross_uid_verified':False,'idle_600_seconds_verified':False,
            'limitations':['Effective systemd declarations and running process privileges checked; active syscall/filesystem denial probes remain separate.',
                           'Two connections here share one UID; no cross-UID or restart/load-pressure qualification claimed.']}),flush=True)
    finally:
        for client in clients:client.socket.close()


if __name__=='__main__':main()
