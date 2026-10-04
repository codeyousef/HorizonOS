"""Coordinate fixed inference tests using two separately enrolled normal UIDs."""
import json
import os
from pathlib import Path
import selectors
import shlex
import subprocess
import time
import uuid

from . import guest, provision, sync
from .errors import DevctlError, ExitCode
from .config import invalid


class Channel:
    def __init__(self,config,role):
        self.config=config;self.identity=guest.enrolled_identity(config)[1]
        _,source=sync.synchronize(config)
        self.verify()
        self.source=sync.contract.decode(Path(source['artifact_path']).read_bytes())
        runner="import runpy,sys; p=sys.argv[1]+'/tools/guest'; sys.path.insert(0,p); sys.argv=[p+'/model_users_probe.py',sys.argv[2]]; runpy.run_path(sys.argv[0],run_name='__main__')"
        command='python3 -I -c '+shlex.quote(runner)+' '+shlex.quote(source['guest_source_path'])+' '+role
        self.process=subprocess.Popen([*guest.ssh_arguments(config)[:-1],command],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        self.buffer=bytearray();self.observations=[];self.stderr=""

    def verify(self):
        if guest.enrolled_identity(self.config)[1]!=self.identity:
            raise invalid('Model qualification target changed')

    def send(self,kind,**fields):
        self.verify();data=(json.dumps({'kind':kind,**fields})+'\n').encode()
        self.process.stdin.write(data);self.process.stdin.flush()

    def read(self,kind,timeout=100):
        deadline=time.monotonic()+timeout
        with selectors.DefaultSelector() as selector:
            selector.register(self.process.stdout,selectors.EVENT_READ)
            while b'\n' not in self.buffer:
                if time.monotonic()>deadline:raise DevctlError(ExitCode.TIMEOUT,'MODEL_USERS_TIMEOUT','Fixed model qualification did not reply')
                for key,_ in selector.select(1):
                    data=os.read(key.fd,4096)
                    if not data:raise invalid('Model qualification ended before its expected reply')
                    self.buffer.extend(data)
                    if len(self.buffer)>65536:raise invalid('Model qualification response exceeded bound')
        line,_,remaining=self.buffer.partition(b'\n');self.buffer=bytearray(remaining);value=json.loads(line)
        if not isinstance(value,dict) or value.get('kind')!=kind:raise invalid('Unexpected fixed model qualification reply')
        self.verify();self.observations.append(value);return value

    def close(self):
        if self.process.poll() is None:
            self.process.stdin.close()
            try:self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:self.process.kill();self.process.wait()
        self.stderr=self.process.stderr.read(8193).decode(errors='replace')
        for pipe in (self.process.stdin,self.process.stdout,self.process.stderr):
            if not pipe.closed:pipe.close()


def run(owner,peer):
    # Every operation uses independent pinned trust; UIDs must differ on one VM.
    _,a=guest.enrolled_identity(owner);_,b=guest.enrolled_identity(peer)
    if a!=b or owner.values['ssh_user']==peer.values['ssh_user']:
        raise invalid('Requires independently enrolled different users on the same guest')
    channels=[]
    directory=provision.private_directory(owner.root,'.local/reports/model-users-'+str(uuid.uuid4()))
    path=directory/'report.json'
    try:
        first=Channel(owner,'owner');channels.append(first);second=Channel(peer,'peer');channels.append(second)
        ready=second.read('peer_ready');answer=first.read('owner_answer')
        if ready['uid']==answer['uid'] or answer['uid']<=0 or ready['uid']<=0:raise invalid('Actual model subjects must have different nonroot UIDs')
        first.send('start_active');active=first.read('owner_active')
        second.send('inspect_foreign',generation_id=answer['generation_id'],active_id=active['generation_id']);queued=second.read('peer_queued')
        first.send('cancel_active');cancelled=first.read('owner_cancelled');result=second.read('peer_answer')
        if cancelled['cancel_ms']>=2000 or answer['private_marker'] in json.dumps(result):raise invalid('Cancellation/privacy acceptance failed')
        first.send('finish');second.send('finish')
        if first.process.wait(timeout=10)!=0 or second.process.wait(timeout=10)!=0:raise invalid('Fixed model qualification failed after results')
        report={'schema_version':1,'evidence_kind':'actual-installed-model-two-real-UIDs','identity':a,'owner_configuration':owner.values,'peer_configuration':peer.values,
            'owner_answer':answer,'owner_active':active,'peer_queued':queued,'owner_cancelled':cancelled,'peer_answer':result,
            'source':{'owner':first.source,'peer':second.source},
            'private_marker_absent_from_other_uid_answer':True,'mutation_performed':False,
            'limitations':['Synthetic private marker and observed native context boundary; not a proof against every possible side channel.',
                           'No full restart/load-pressure, 600-second idle, installed filesystem denial or memory peak acceptance.']}
        provision.write_json_new(path,report)
        return ExitCode.SUCCESS,{'artifact_path':str(path),'identity_verified':True,'owner_uid':answer['uid'],'peer_uid':ready['uid'],'cancel_ms':cancelled['cancel_ms']}
    except (DevctlError,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as error:
        for channel in channels:channel.close()
        provision.write_json_new(path,{'schema_version':1,'state':'failed','evidence_kind':'actual-fixed-model-two-user-attempt',
            'identity':a,'failure_type':type(error).__name__,'failure':str(error),
            'channels':[{'configuration':v.config.values,'identity':v.identity,'source':v.source,'observations':v.observations,
                         'upstream_exit':v.process.returncode,'stderr':v.stderr} for v in channels]})
        channels=[]
        raise DevctlError(ExitCode.VERIFICATION_FAILURE,'MODEL_USERS_FAILED','Fixed installed two-user qualification failed; evidence retained',details={'artifact_path':str(path)}) from error
    finally:
        for channel in channels:channel.close()
