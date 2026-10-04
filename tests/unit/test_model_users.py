"""Host-only target-confusion fixtures; no inference or privacy certification."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools'))
from aios_dev import cli,model_users
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError


class ModelUsersTests(unittest.TestCase):
    def test_different_guests_and_same_user_fail_before_source_or_launch(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);values=json.loads((ROOT/'dev/vm.example.json').read_text())
            owner=VMConfig.from_data(root,values)
            peer=VMConfig.from_data(root,{**values,'ssh_user':'tester','guest_source_root':'/home/tester/aios-releases'})
            for target,identities in [(peer,[({}, {'dmi_uuid':'a'}),({}, {'dmi_uuid':'b'})]),(owner,[({}, {'dmi_uuid':'a'}),({}, {'dmi_uuid':'a'})])]:
                with patch('aios_dev.model_users.guest.enrolled_identity',side_effect=identities),patch('aios_dev.model_users.sync.synchronize') as sync,patch('aios_dev.model_users.subprocess.Popen') as start:
                    with self.assertRaises(DevctlError):model_users.run(owner,target)
                    sync.assert_not_called();start.assert_not_called()

    def test_missing_peer_and_detached_two_user_requests_are_denied(self):
        for args in [('test','--suite','integration','--provider','installed-model-users'),
                     ('test','--suite','integration','--provider','installed-model-users','--peer-workspace','/unused','--detach'),
                     ('test','--suite','unit','--peer-workspace','/unused')]:
            with patch('aios_dev.cli.load_config') as config,self.assertRaises(DevctlError):
                cli.dispatch(cli.parser().parse_args(args))
            config.assert_not_called()

    def test_invalid_response_retains_failed_attempt_and_closes_connections(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);values=json.loads((ROOT/'dev/vm.example.json').read_text())
            owner=VMConfig.from_data(root,values)
            peer=VMConfig.from_data(root,{**values,'ssh_user':'tester','guest_source_root':'/home/tester/aios-releases'})
            identity={'dmi_uuid':'fixture'}
            first=Mock(config=owner,identity=identity,source={'fixture':True},observations=[],stderr='fixture failure')
            second=Mock(config=peer,identity=identity,source={'fixture':True},observations=[],stderr='')
            first.process.returncode=1;second.process.returncode=0
            second.read.return_value={'kind':'peer_ready','uid':1001}
            first.read.return_value={'kind':'owner_answer'}
            with patch('aios_dev.model_users.guest.enrolled_identity',return_value=({},identity)),patch('aios_dev.model_users.Channel',side_effect=[first,second]):
                with self.assertRaises(DevctlError) as caught:model_users.run(owner,peer)
            first.close.assert_called_once();second.close.assert_called_once()
            report=json.loads(Path(caught.exception.details['artifact_path']).read_text())
            self.assertEqual(report['state'],'failed')
            self.assertEqual(report['failure_type'],'KeyError')
            self.assertEqual(report['channels'][0]['upstream_exit'],1)

if __name__=='__main__':unittest.main()
