"""Real file/lease boundary fixtures; not OS recovery acceptance."""
import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from aios_dev import snapshots, provision, vm
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError, ExitCode

EXAMPLE = json.loads((ROOT / 'dev/vm.example.json').read_text())
TARGET = {'guest_uuid': '22222222-2222-4222-8222-222222222222', 'installation_uuid': '33333333-3333-4333-8333-333333333333', 'guest_role': 'development'}


class SnapshotTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.config = VMConfig.from_data(self.root, EXAMPLE)
        provision.private_directory(self.root, '.local/vm')
        for field in snapshots.FIELDS:
            provision.write_new(self.config.paths[field], (field + '-initial').encode())
        info = self.config.paths['disk_image'].stat()
        self.record = {'disk_device': info.st_dev, 'disk_inode': info.st_ino}

    def directory(self, name):
        return self.root / '.local/vm/snapshots' / name

    def operation(self, name='baseline'):
        return patch.multiple(snapshots, location=lambda c,n:self.directory(n), binding=lambda c:(self.record,TARGET),
                              qcow_info=lambda c:{'format':'qcow2','virtual_size':8192})

    def test_live_descriptor_denies_snapshot_without_completed_manifest(self):
        with self.operation(), self.config.paths['disk_image'].open('rb'):
            with self.assertRaises(DevctlError) as caught:
                snapshots.snapshot(self.config, 'baseline')
        self.assertEqual(caught.exception.code, 'IMAGE_NOT_UNUSED')
        self.assertFalse((self.directory('baseline') / 'manifest.json').exists())

    def test_new_open_breaks_lease_and_aborts(self):
        path = self.config.paths['disk_image']
        with self.assertRaises(DevctlError) as caught:
            with snapshots.unused_files([path]) as (_, check):
                with self.assertRaises(BlockingIOError):
                    os.open(path, os.O_RDONLY | os.O_NONBLOCK)
                check()
        self.assertEqual(caught.exception.code, 'IMAGE_LEASE_BROKEN')

    def test_sparse_copy_hashes_and_preserves_holes(self):
        path = self.config.paths['disk_image']
        with path.open('r+b') as f:
            f.seek(16 * 1024 * 1024); f.write(b'end')
        dest = self.root / 'sparse'
        fd = os.open(dest, os.O_RDWR | os.O_CREAT | os.O_EXCL, 0o600)
        self.addCleanup(os.close, fd)
        with snapshots.unused_files([path]) as (fds, check):
            snapshots.sparse_copy(fds[path], fd, check)
            self.assertEqual(snapshots.fd_digest(fds[path], check), snapshots.fd_digest(fd, check))
        self.assertEqual(dest.stat().st_size, path.stat().st_size)
        self.assertLess(dest.stat().st_blocks * 512, dest.stat().st_size // 2)

    def test_snapshot_restore_preserves_inode_and_external_reports(self):
        reports = provision.private_directory(self.root, '.local/reports') / 'retained.json'
        provision.write_new(reports, b'host evidence')
        with self.operation():
            code, result = snapshots.snapshot(self.config, 'baseline')
            self.assertEqual(code, 0)
            for field in snapshots.FIELDS:
                self.config.paths[field].write_bytes(b'changed guest image')
            code, restored = snapshots.restore(self.config, 'baseline', True)
        self.assertEqual(code, 0)
        self.assertTrue(restored['discarded_guest_changes'])
        self.assertFalse(restored['approvals_revalidated'])
        self.assertEqual(reports.read_bytes(), b'host evidence')
        self.assertEqual(self.config.paths['disk_image'].stat().st_ino, self.record['disk_inode'])
        for field in snapshots.FIELDS:
            self.assertEqual(self.config.paths[field].read_bytes(), (field + '-initial').encode())
        self.assertFalse((self.root / '.local/vm/restore-pending.json').exists())

    def test_corrupt_manifest_or_artifact_denied_before_restore(self):
        for victim in ('manifest.json','root.qcow2'):
            name = victim.replace('.', '_')
            with self.operation():
                snapshots.snapshot(self.config, name)
                path = self.directory(name) / victim
                path.write_bytes(path.read_bytes() + b'corruption')
                before = self.config.paths['disk_image'].read_bytes()
                with self.assertRaises(DevctlError):
                    snapshots.restore(self.config, name, True)
                self.assertEqual(self.config.paths['disk_image'].read_bytes(), before)

    def test_restore_authorization_and_pending_boot_denial(self):
        with self.assertRaises(DevctlError) as caught:
            snapshots.restore(self.config, 'baseline', False)
        self.assertEqual(caught.exception.exit_code, ExitCode.AUTHORIZATION_NEEDED)
        provision.write_json_new(self.root / '.local/vm/restore-pending.json', {'interrupted':True})
        with self.assertRaises(DevctlError) as caught:
            vm.start(self.config, 'none', False)
        self.assertEqual(caught.exception.code, 'RESTORE_INCOMPLETE')

    def test_interrupted_restore_resumes_same_snapshot_only(self):
        original = snapshots.sparse_copy
        with self.operation():
            snapshots.snapshot(self.config, 'baseline')
            snapshots.snapshot(self.config, 'other')
            self.config.paths['disk_image'].write_bytes(b'changed')
            def interrupted(source, target, check):
                os.ftruncate(target, 0)
                raise OSError('fixture interrupted write')
            with patch.object(snapshots, 'sparse_copy', interrupted):
                with self.assertRaises(OSError):
                    snapshots.restore(self.config, 'baseline', True)
            self.assertTrue((self.root / '.local/vm/restore-pending.json').exists())
            with self.assertRaises(DevctlError):
                snapshots.restore(self.config, 'other', True)
            snapshots.restore(self.config, 'baseline', True)
            self.assertFalse((self.root / '.local/vm/restore-pending.json').exists())
            self.assertEqual(self.config.paths['disk_image'].read_bytes(), b'disk_image-initial')

    def test_snapshot_name_and_replaced_disk_denied(self):
        for name in ('../escape','a/b','.', 'x'*65):
            with self.assertRaises(DevctlError):
                snapshots.location(self.config, name)
        with self.operation():
            snapshots.snapshot(self.config, 'baseline')
            self.record['disk_inode'] += 1
            with self.assertRaises(DevctlError):
                snapshots.restore(self.config, 'baseline', True)
