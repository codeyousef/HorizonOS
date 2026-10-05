"""Fixed root coordinator; observations originate in a normal installed client.

Only compiled disposable-image units are controlled. No arguments, RPC, caller
paths, model decisions or sudo route can select an effect.
"""
import hashlib
import os
from pathlib import Path
import stat
import subprocess
import time

UNITS = ('aios-service-failure-fixture.service', 'aios-service-restart-fixture.service')
SYSTEMCTL = '/run/current-system/sw/bin/systemctl'


def observation(value, expected, unit):
    result = value['result']
    if (value['upstream_exit'] != 0 or result['status'] != 'ok' or result['complete'] is not True
            or result['error'] is not None or result['source']['provider'] != 'systemd'):
        raise RuntimeError('fixture service observation was incomplete')
    data = result['data']
    if (data['unit_name'] != unit or data['scope'] != 'system' or data['boot_id'] != expected['boot_id']
            or data['ordering_is_not_causation'] is not True or 'sshd.service' not in data['ordering_after']):
        raise RuntimeError('fixture service target or ordering provenance differs')
    return data


def qualified(report, expected):
    """Reject missing cases, stale observations and failed owned cleanup."""
    try:
        if (report['schema_version'] != 1 or report['verified'] is not True
                or report['evidence_kind'] != 'actual-installed-fixed-service-startup-and-failures'):
            return False
        startup = observation(report['observations']['startup'], expected, UNITS[0])
        failed = observation(report['observations']['failed'], expected, UNITS[0])
        restarted = observation(report['observations']['restart_exhausted'], expected, UNITS[1])
        job = startup['job']
        if (startup['active_state'] != 'activating' or startup['sub_state'] != 'start'
                or type(job['id']) is not int or job['id'] <= 0 or job['job_type'] != 'start'
                or job['state'] != 'running' or job['object_path'] != '/org/freedesktop/systemd1/job/'+str(job['id'])
                or not startup['invocation_id'] or startup['invocation_id'] != failed['invocation_id']):
            return False
        for data, result in ((failed, 'exit-code'), (restarted, 'start-limit-hit')):
            if (data['active_state'] != 'failed' or data['result'] != result or data['exec_main_status'] != 7
                    or data['job'] is not None or data['main_pid'] != 0):
                return False
        if (failed['restart_count'] != 0 or not 2 <= restarted['restart_count'] <= 4
                or restarted['restart_count'] != int(report['independent_restart_unit']['NRestarts'])):
            return False
        refusal = report['scope_refusals']
        if refusal['mutation_performed'] is not False:
            return False
        for key, code in (('foreign_stream', 'PERMISSION_DENIED'), ('missing_service', 'TARGET_NOT_FOUND')):
            if refusal[key]['error']['code'] != code or refusal[key]['data'] is not None:
                return False
        for unit in UNITS:
            state = report['units'][unit]['after_cleanup']
            if state['ActiveState'] != 'inactive' or state['SubState'] != 'dead':
                return False
        controls = [(row['argv'], row['upstream_exit']) for row in report['controls']]
        required = [([SYSTEMCTL, '--no-block', 'start', unit], 0) for unit in UNITS]
        required += [([SYSTEMCTL, operation, unit], 0) for unit in UNITS for operation in ('stop', 'reset-failed')]
        if len(controls) != len(required) or any(row not in controls for row in required):
            return False
        for field in ('boot_id', 'main_pid', 'active_state', 'sub_state', 'restart_count'):
            if report['health_before']['service']['data'][field] != report['health_after']['service']['data'][field]:
                return False
        return True
    except (KeyError, TypeError, ValueError, RuntimeError):
        return False


def run(expected, recheck, public, report):
    if os.getuid() != 0 or os.geteuid() != 0:
        raise PermissionError('fixed service coordinator requires initial root fixture')
    recheck(expected)
    profile = Path('/etc/aios/service-fixture-profile')
    resolved = profile.resolve(strict=True)
    info = resolved.stat()
    if (not resolved.is_relative_to('/nix/store') or info.st_uid != 0 or info.st_mode & 0o222
            or profile.read_text().strip() != 'fixed-native-service-failures-v1'):
        raise RuntimeError('not the compiled service failure test image')
    report.update({'schema_version':1,'evidence_kind':'actual-installed-fixed-service-startup-and-failures',
                   'verified':False,'controls':[],'units':{},'observations':{}})

    def show(unit):
        if unit not in UNITS: raise ValueError('unregistered fixture unit')
        recheck(expected)
        result = subprocess.run([SYSTEMCTL,'show',unit,'--property=LoadState','--property=ActiveState',
            '--property=SubState','--property=Result','--property=NRestarts','--property=ExecMainStatus',
            '--property=FragmentPath','--property=User','--property=NoNewPrivileges','--property=CapabilityBoundingSet'],
            capture_output=True,text=True,check=True,timeout=5)
        recheck(expected)
        if len(result.stdout)+len(result.stderr)>32768: raise RuntimeError('fixture unit read exceeds bound')
        return dict(line.split('=',1) for line in result.stdout.splitlines() if '=' in line)

    def control(operation, unit):
        if unit not in UNITS or operation not in ('start','stop','reset-failed'): raise ValueError('unregistered fixture control')
        recheck(expected)
        arguments=[SYSTEMCTL]+(['--no-block'] if operation=='start' else [])+[operation,unit]
        result=subprocess.run(arguments,capture_output=True,timeout=5)
        recheck(expected)
        report['controls'].append({'argv':arguments,'upstream_exit':result.returncode})
        if result.returncode or len(result.stdout)+len(result.stderr)>32768: raise RuntimeError('fixed service control failed')

    # Prove these are newly owned inactive fixtures before any control. Never
    # stop/reset a substituted or already active unit during cleanup.
    for unit in UNITS:
        state=show(unit);path=Path(state['FragmentPath']).resolve(strict=True);info=path.stat()
        if (state['LoadState']!='loaded' or state['ActiveState']!='inactive' or state['SubState']!='dead'
                or state['NRestarts']!='0' or state['User']!='nobody' or state['NoNewPrivileges']!='yes'
                or state['CapabilityBoundingSet']!='' or path.name!=unit or not path.is_relative_to('/nix/store')
                or not stat.S_ISREG(info.st_mode) or info.st_uid!=0 or info.st_mode & 0o222):
            raise RuntimeError('fixture unit is not inactive protected compiled data')
        report['units'][unit]={'initial':state,'resolved_fragment':str(path),'fragment_sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
    owned=[]
    try:
        before=public.call('health');report['health_before']=before
        owned.append(UNITS[0]);control('start',UNITS[0])
        deadline=time.monotonic()+3
        while show(UNITS[0])['SubState']!='start':
            if time.monotonic()>=deadline: raise RuntimeError('oneshot fixture did not begin its startup job')
            time.sleep(0.02)
        pending=public.call('fixture_service');report['observations']['startup']=pending
        data=observation(pending,expected,UNITS[0]);job=data['job']
        if (data['active_state']!='activating' or data['sub_state']!='start' or not job or job['id']<=0
                or job['job_type']!='start' or job['state']!='running'
                or job['object_path']!='/org/freedesktop/systemd1/job/'+str(job['id'])):
            raise RuntimeError('actual startup job was not observed')
        deadline=time.monotonic()+15
        while show(UNITS[0])['ActiveState']!='failed':
            if time.monotonic()>=deadline: raise RuntimeError('oneshot fixture did not reach its fixed failure')
            time.sleep(0.05)
        failed=public.call('fixture_service');report['observations']['failed']=failed
        data=observation(failed,expected,UNITS[0])
        if (data['active_state']!='failed' or data['result']!='exit-code' or data['exec_main_status']!=7
                or data['main_pid']!=0 or data['job'] is not None or data['restart_count']!=0):
            raise RuntimeError('fixed failed service was reported inaccurately')
        owned.append(UNITS[1]);control('start',UNITS[1]);deadline=time.monotonic()+15
        while show(UNITS[1])['ActiveState']!='failed':
            if time.monotonic()>=deadline: raise RuntimeError('restart fixture did not exhaust its bounded starts')
            time.sleep(0.05)
        restarted=public.call('fixture_restart_service');report['observations']['restart_exhausted']=restarted
        data=observation(restarted,expected,UNITS[1]);native=show(UNITS[1]);report['independent_restart_unit']=native
        if (data['active_state']!='failed' or data['result']!='start-limit-hit' or data['exec_main_status']!=7
                or not 2<=data['restart_count']<=4 or data['restart_count']!=int(native['NRestarts'])
                or data['job'] is not None or data['main_pid']!=0):
            raise RuntimeError('native restart/failure counter does not match installed observation')
        report['scope_refusals']=public.call('fixture_scope')
        after=public.call('health');report['health_after']=after
        for field in ('boot_id','main_pid','active_state','sub_state','restart_count'):
            if before['service']['data'][field]!=after['service']['data'][field]:
                raise RuntimeError('deliberate service failure affected the management service')
    finally:
        for unit in owned:
            current=show(unit);original=report['units'][unit]
            path=Path(current['FragmentPath']).resolve(strict=True)
            if (str(path)!=original['resolved_fragment'] or hashlib.sha256(path.read_bytes()).hexdigest()!=original['fragment_sha256']):
                report['verified']=False;raise RuntimeError('fixture ownership changed; cleanup refused')
            control('stop',unit);control('reset-failed',unit)
            report['units'][unit]['after_cleanup']=show(unit)
            if report['units'][unit]['after_cleanup']['ActiveState']!='inactive':
                raise RuntimeError('owned fixture did not become inactive')
        recheck(expected)
    # Cleanup is part of qualification, including its final target identity.
    # An exception anywhere above leaves the nested proof unverified.
    report['verified']=True
