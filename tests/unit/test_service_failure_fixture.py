"""Synthetic gate/refusal cases; these are not native systemd evidence."""
import copy
from pathlib import Path
import sys
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools/guest'))
import service_failure_fixture as fixture


def proof():
    def read(unit, **fields):
        return {'upstream_exit':0,'result':{'status':'ok','complete':True,'error':None,
            'source':{'provider':'systemd'},'data':{'unit_name':unit,'scope':'system','boot_id':'boot',
            'ordering_is_not_causation':True,'ordering_after':['sshd.service'],'invocation_id':'1'*32,
            'active_state':'failed','sub_state':'failed','result':'exit-code','exec_main_status':7,
            'main_pid':0,'job':None,'restart_count':0,**fields}}}
    health={'service':{'data':{'boot_id':'boot','main_pid':42,'active_state':'active','sub_state':'running','restart_count':0}}}
    return {'schema_version':1,'evidence_kind':'actual-installed-fixed-service-startup-and-failures','verified':True,
        'observations':{'startup':read(fixture.UNITS[0],active_state='activating',sub_state='start',
            job={'id':3,'object_path':'/org/freedesktop/systemd1/job/3','job_type':'start','state':'running'}),
            'failed':read(fixture.UNITS[0]),'restart_exhausted':read(fixture.UNITS[1],result='start-limit-hit',restart_count=3)},
        'independent_restart_unit':{'NRestarts':'3'},
        'scope_refusals':{'mutation_performed':False,'foreign_stream':{'error':{'code':'PERMISSION_DENIED'},'data':None},
            'missing_service':{'error':{'code':'TARGET_NOT_FOUND'},'data':None}},
        'units':{unit:{'after_cleanup':{'ActiveState':'inactive','SubState':'dead'}} for unit in fixture.UNITS},
        'controls':[{'argv':[fixture.SYSTEMCTL,'--no-block','start',unit],'upstream_exit':0} for unit in fixture.UNITS]
            +[{'argv':[fixture.SYSTEMCTL,op,unit],'upstream_exit':0} for unit in fixture.UNITS for op in ('stop','reset-failed')],
        'health_before':copy.deepcopy(health),'health_after':copy.deepcopy(health)}


class FailureQualificationTests(unittest.TestCase):
    def test_required_cases_cannot_be_missing_or_verified_flag_only(self):
        self.assertTrue(fixture.qualified(proof(), {'boot_id':'boot'}))
        self.assertFalse(fixture.qualified({'verified':True}, {'boot_id':'boot'}))
        for field in proof():
            value=proof();del value[field]
            self.assertFalse(fixture.qualified(value, {'boot_id':'boot'}), field)
        for row in ('startup','failed','restart_exhausted'):
            value=proof();del value['observations'][row]
            self.assertFalse(fixture.qualified(value, {'boot_id':'boot'}), row)

    def test_stale_partial_healthy_empty_and_false_cleanup_do_not_pass(self):
        mutations=[(('verified',),False),(('evidence_kind',),'synthetic'),
            (('observations','failed','result','complete'),False),
            (('observations','failed','result','data','boot_id'),'previous-boot'),
            (('observations','failed','result','data','invocation_id'),'2'*32),
            (('observations','startup','result','data','job'),None),
            (('observations','restart_exhausted','result','data','restart_count'),0),
            (('independent_restart_unit','NRestarts'),'4'),
            (('scope_refusals','foreign_stream','data'),{}),
            (('scope_refusals','missing_service','error','code'),'OK'),
            (('units',fixture.UNITS[0],'after_cleanup','ActiveState'),'failed'),
            (('health_after','service','data','main_pid'),43),
            (('controls',),[])]
        for path, replacement in mutations:
            value=proof();target=value
            for key in path[:-1]:target=target[key]
            target[path[-1]]=replacement
            self.assertFalse(fixture.qualified(value, {'boot_id':'boot'}), str(path))

    def test_nonroot_never_reaches_target_or_service_controls(self):
        for real,effective in ((1000,1000),(0,1000),(1000,0)):
            target=Mock()
            with patch.object(fixture.os,'getuid',return_value=real),patch.object(fixture.os,'geteuid',return_value=effective), \
                    patch.object(fixture.subprocess,'run') as control:
                with self.assertRaises(PermissionError):fixture.run({},target,Mock(),{})
                target.assert_not_called();control.assert_not_called()


if __name__=='__main__':unittest.main()
