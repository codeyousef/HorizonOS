"""Boundary fixtures only; installed public crash/hash failures run in the VM."""
from pathlib import Path
import copy
import sys
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools/guest'))
import model_public_fixture as fixture
import model_lifecycle_preflight as root_fixture


class PublicFailureBoundaryTests(unittest.TestCase):
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
