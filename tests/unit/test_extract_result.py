"""Host negative report fixtures; not native extraction acceptance."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('extract_result', Path(__file__).resolve().parents[2] / 'tools/guest/extract_result.py')
v = importlib.util.module_from_spec(spec)
spec.loader.exec_module(v)


class ExtractReportTests(unittest.TestCase):
    def setUp(self):
        self.result = dict(schema_version=1,evidence_kind='installed-descriptor-extractor-synthetic-corpus',
            production_index_integrated=False,
            identity=dict(dmi_uuid='3a491b68-9b63-4d24-b232-8d7ce1ba044c',installation_uuid='699932ee-271f-429c-b478-edb05b33c5d4',
                disk_serial='root',guest_role='development',management_channel='nixos-test-driver',
                boot_id='a076ee84-a48f-4df2-80c8-ae154f089cce',machine_id='a'*32,
                host_key_fingerprint='SHA256:'+'a'*43,current_system='/nix/store/'+'a'*32+'-nixos-system-machine-test'),
            worker_security=dict(uid=1000,nnp=1,seccomp=2,capabilities_empty=True,private_namespaces=True,pid=123,
                executable='/nix/store/fixture/bin/aios-extract'),
            resource_probes={name:dict(Result=status,MemoryMax='536870912',MemorySwapMax='0',RuntimeMaxUSec='10s',CPUQuotaPerSecUSec='900ms',TasksMax='8',KillMode='control-group',CPUUsageNSec='9000000000')
                for name,status in [('memory','oom-kill'),('cpu','timeout')]})
        self.result.update({case:True for case in v.CASES})

    def test_each_required_case_is_mandatory(self):
        v.validate(self.result)
        for case in v.CASES:
            for value in (None,False,'true',1):
                changed=copy.deepcopy(self.result);changed[case]=value
                with self.subTest(case=case,value=value),self.assertRaises(ValueError):v.validate(changed)

    def test_wrong_target_or_overclaim_refuses(self):
        for field,value in [('guest_role','host'),('disk_serial','unknown'),('installation_uuid','other')]:
            changed=copy.deepcopy(self.result);changed['identity'][field]=value
            with self.assertRaises(ValueError):v.validate(changed)
        changed=copy.deepcopy(self.result);changed['production_index_integrated']=True
        with self.assertRaises(ValueError):v.validate(changed)

    def test_missing_native_protection_or_resource_limit_refuses(self):
        for field,value in [('nnp',0),('seccomp',0),('capabilities_empty',False),('private_namespaces',False),('pid',0)]:
            changed=copy.deepcopy(self.result);changed['worker_security'][field]=value
            with self.assertRaises(ValueError):v.validate(changed)
        for field,value in [('Result','success'),('MemoryMax','infinity'),('MemorySwapMax','infinity'),('KillMode','process'),('RuntimeMaxUSec','infinity'),('CPUQuotaPerSecUSec','infinity')]:
            changed=copy.deepcopy(self.result);changed['resource_probes']['memory'][field]=value
            with self.assertRaises(ValueError):v.validate(changed)
        changed=copy.deepcopy(self.result);changed['resource_probes']['cpu']['CPUUsageNSec']='10000000001'
        with self.assertRaises(ValueError):v.validate(changed)
