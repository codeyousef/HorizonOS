"""Host lifecycle fixtures; these do not certify recovered guest storage."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch, MagicMock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from aios_dev import vm
from aios_dev.cli import parser
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError


class StorageResumeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.config = VMConfig.from_data(Path(self.temp.name), json.loads((ROOT / 'dev/vm.example.json').read_text()))
        self.block = {'device': 'rootdisk', 'io-status': 'nospace',
                      'inserted': {'file': str(self.config.paths['disk_image'])}}
        self.statuses = [{'status': 'io-error', 'running': False}, {'status': 'running', 'running': True}]
        self.client = MagicMock()
        self.client.command.side_effect = self.command
        for name, value in [('load_record', {'plan': {'guest_uuid': 'owned-uuid'}}),
                            ('read_json', {'pid': 42}), ('QMP', self.client)]:
            p = patch.object(vm, name, return_value=value); p.start(); self.addCleanup(p.stop)
        p = patch('aios_dev.resources.require_build_headroom'); self.headroom = p.start(); self.addCleanup(p.stop)

    def command(self, name):
        if name == 'query-block':
            return [copy.deepcopy(self.block)]
        if name == 'query-status':
            return self.statuses.pop(0)
        raise AssertionError(name)

    def test_resume_retains_process_and_requires_fresh_guest_identity(self):
        code, report = vm.resume_storage(self.config)
        self.assertEqual(code, 0); self.assertEqual(report['pid'], 42)
        self.assertFalse(report['guest_identity_verified'])
        self.client._request.assert_called_once_with('cont')
        self.assertEqual(self.headroom.call_count, 2)
        self.client.close.assert_called_once()
        self.assertEqual(parser().parse_args(['vm', 'resume-storage']).operation, 'resume-storage')

    def test_low_disk_never_releases_pending_writes(self):
        self.headroom.side_effect = DevctlError(3, 'LOW_DISK', 'fixture')
        with self.assertRaises(DevctlError): vm.resume_storage(self.config)
        self.client._request.assert_not_called()

    def test_running_other_error_and_wrong_disk_never_resume(self):
        for status, reason, filename in [('running', 'nospace', str(self.config.paths['disk_image'])),
                                         ('io-error', 'failed', str(self.config.paths['disk_image'])),
                                         ('io-error', 'nospace', '/another/disk')]:
            self.statuses = [{'status': status, 'running': status == 'running'}]
            self.block['io-status'] = reason; self.block['inserted']['file'] = filename
            with self.subTest(status=status, reason=reason, filename=filename), self.assertRaises(DevctlError):
                vm.resume_storage(self.config)
            self.client._request.assert_not_called()

    def test_second_reserve_check_failure_does_not_resume(self):
        self.headroom.side_effect = [None, DevctlError(3, 'LOW_DISK', 'fixture')]
        with self.assertRaises(DevctlError): vm.resume_storage(self.config)
        self.client._request.assert_not_called(); self.client.close.assert_called_once()

    def test_secondary_device_error_and_qmp_identity_failure_never_resume(self):
        self.client.command.side_effect = lambda name: (
            [self.block, {'device': 'pflash1', 'io-status': 'failed'}]
            if name == 'query-block' else {'status': 'io-error', 'running': False})
        with self.assertRaises(DevctlError): vm.resume_storage(self.config)
        self.client._request.assert_not_called()
        with patch.object(vm, 'QMP', side_effect=DevctlError(4, 'QMP_PEER_MISMATCH', 'fixture')):
            with self.assertRaises(DevctlError): vm.resume_storage(self.config)
        self.client._request.assert_not_called()

    def test_incomplete_restore_and_post_resume_failure_are_not_success(self):
        pending = self.config.root / '.local/vm/restore-pending.json'
        pending.parent.mkdir(parents=True); pending.write_text('{}')
        with self.assertRaises(DevctlError): vm.resume_storage(self.config)
        self.client._request.assert_not_called()
        pending.unlink()
        self.statuses[1] = {'status': 'io-error', 'running': False}
        with self.assertRaises(DevctlError) as error: vm.resume_storage(self.config)
        self.assertEqual(error.exception.code, 'VM_STORAGE_RESUME_FAILED')


if __name__ == '__main__': unittest.main()
