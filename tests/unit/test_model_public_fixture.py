"""Boundary fixtures only; installed public crash/hash failures run in the VM."""
from pathlib import Path
import copy
import sys
import subprocess
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools/guest'))
import model_public_fixture as fixture
import model_lifecycle_preflight as root_fixture


class PublicFailureBoundaryTests(unittest.TestCase):
    def broker(self):
        broker = fixture.Broker.__new__(fixture.Broker)
        broker.uid = 1001
        broker.unit = 'aios-sessiond.service'
        broker.env = {}
        broker.bytes = b'original packaged unit'
        broker.binaries = {'aios-sessiond': Path('/nix/store/' + 'a'*32 + '-aios-core-0.1.0/bin/aios-sessiond')}
        binary = str(broker.binaries['aios-sessiond'])
        broker.show = Mock(return_value={'NoNewPrivileges':'yes','PrivateNetwork':'yes','ProtectHome':'tmpfs',
            'ProtectSystem':'strict','MemoryMax':'268435456','TasksMax':'64','RuntimeDirectoryMode':'0700',
            'ActiveState':'active','MainPID':'50','FragmentPath':'/protected/unit','DropInPaths':'/protected/dropin',
            'ExecStart':'{ path=' + binary + ' ; argv[]=' + binary + ' ; ignore_errors=no ; rest }',
            'InvocationID':'a'*32})
        broker.protected_file = Mock(return_value=Mock(read_bytes=Mock(return_value=broker.bytes)))
        broker.bus = Mock(side_effect=lambda *args: b'u 1001\n' if args[-3] == 'GetConnectionUnixUser'
                          else b'u 51\n' if args[-1] == 'org.freedesktop.systemd1' else b'u 50\n')
        return broker

    def test_original_service_attestation_refuses_drift_and_replacement(self):
        broker = self.broker()
        native = '50 (broker) ' + ' '.join(['S'] + ['0']*18 + ['123'])
        with patch.object(fixture.subprocess, 'run', return_value=subprocess.CompletedProcess([],0,b'51\n')), \
                patch.object(Path, 'read_text', autospec=True, side_effect=lambda *args, **kw: native if args[0].name == 'stat' else 'boot'):
            broker.identity = broker.verify()
            broker.verify()
            for key, value in [('MainPID','52'),('InvocationID','b'*32),('ActiveState','inactive'),
                               ('ExecStart',broker.show.return_value['ExecStart'].replace('ignore_errors=no','--socket /tmp/fake ; ignore_errors=no')),
                               ('ProtectSystem','no')]:
                previous = broker.show.return_value[key]
                broker.show.return_value[key] = value
                with self.assertRaises(RuntimeError): broker.verify()
                broker.show.return_value[key] = previous
            broker.protected_file.return_value.read_bytes.return_value = b'replaced'
            with self.assertRaises(RuntimeError): broker.verify()
        broker = self.broker()
        with patch.object(fixture.subprocess, 'run', return_value=subprocess.CompletedProcess([],0,b'999\n')):
            with self.assertRaisesRegex(RuntimeError, 'root manager'): broker.verify()

    def test_cleanup_owns_only_its_cli_and_never_stops_the_shared_broker(self):
        broker = self.broker()
        broker.cli = Mock()
        broker.cli.poll.return_value = None
        child = broker.cli
        broker.verify = Mock()
        broker.ctl = Mock()
        broker.close()
        child.kill.assert_called_once_with()
        child.communicate.assert_called_once_with(timeout=5)
        self.assertIsNone(broker.cli)
        broker.verify.assert_called_once_with()
        broker.ctl.assert_not_called()

    def test_root_coordinator_checks_fixed_tester_before_forking(self):
        account = Mock(pw_uid=1000)
        with patch.object(fixture.os, 'getuid', return_value=0), patch.object(fixture.os, 'geteuid', return_value=0), \
                patch.object(fixture.pwd, 'getpwnam', return_value=account), patch.object(fixture.os, 'fork') as fork:
            with self.assertRaisesRegex(RuntimeError, 'tester identity'): fixture.PublicProbe({}, Mock())
        fork.assert_not_called()

    def test_recovery_accepts_brand_case_but_requires_current_native_citations(self):
        boot = '00000000-0000-0000-0000-000000000001'
        base = {'upstream_exit':0,'answer':{'state':'completed','error':None,'mutation_performed':False,
                'output':{'local_cpu':True,'mutation_performed':False,'response':{'kind':'answer','text':'nixos','evidence_ids':['fresh']},
                'evidence':[{'complete':True,'error':None,'source':{'provider':'aios-system'},
                             'data':{'os_id':'nixos','boot_id':boot},'evidence_ids':['fresh']}]}}}
        for text in ['nixos','NixOS','The OS is NIXOS.']:
            value=copy.deepcopy(base);value['answer']['output']['response']['text']=text
            fixture.require_system_answer(value,boot)
        for field,value in [('local_cpu',False),('mutation_performed',True),('evidence',[])]:
            failed=copy.deepcopy(base);failed['answer']['output'][field]=value
            with self.assertRaises(RuntimeError):fixture.require_system_answer(failed,boot)
        for field,value in [('kind','abstain'),('text','notnixos'),('evidence_ids',[]),('evidence_ids',['old'])]:
            failed=copy.deepcopy(base);failed['answer']['output']['response'][field]=value
            with self.assertRaises(RuntimeError):fixture.require_system_answer(failed,boot)
        for field,value in [('complete',False),('source',{'provider':'fixture'}),('data',{'os_id':'nixos','boot_id':'another-boot'})]:
            failed=copy.deepcopy(base);failed['answer']['output']['evidence'][0][field]=value
            with self.assertRaises(RuntimeError):fixture.require_system_answer(failed,boot)

    def test_nonroot_cannot_enter_coordinator_or_corrupt_mount_control(self):
        for real, effective in ((1000, 1000), (0, 1000), (1000, 0)):
            target = Mock()
            with patch.object(fixture.os, 'getuid', return_value=real), patch.object(fixture.os, 'geteuid', return_value=effective), \
                    patch.object(fixture.os, 'fork') as fork, patch.object(fixture.pwd, 'getpwnam') as account:
                with self.assertRaises(PermissionError): fixture.PublicProbe({}, target)
                with self.assertRaises(PermissionError): root_fixture.corrupt_model({}, Mock())
                target.assert_not_called(); account.assert_not_called(); fork.assert_not_called()

    def test_failure_requires_exact_code_terminal_state_and_no_fabricated_output(self):
        base = {'upstream_exit': 1, 'answer': {'state': 'failed', 'error': 'MODEL_CRASHED', 'output': None, 'mutation_performed': False}}
        fixture.require_failure(base, 'MODEL_CRASHED')
        for key, value in (('state', 'completed'), ('error', 'MODEL_UNAVAILABLE'), ('output', {'text': 'fabricated answer'}), ('mutation_performed', True)):
            with self.assertRaises(RuntimeError): fixture.require_failure({**base, 'answer': {**base['answer'], key: value}}, 'MODEL_CRASHED')
        with self.assertRaises(RuntimeError): fixture.require_failure({**base, 'upstream_exit': 0}, 'MODEL_CRASHED')


if __name__ == '__main__':
    unittest.main()
