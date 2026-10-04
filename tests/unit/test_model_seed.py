"""Host model-delivery denial fixtures; no installed-model acceptance claim."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools'))
from aios_dev import model_seed, provision
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError


class ModelSeedTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name); (self.root/'models').mkdir()
        (self.root/'models/lock.json').write_text('{}')
        self.data=b'public weights fixture'
        self.values=[{'name':'model.gguf','remote':'model.gguf','bytes':len(self.data),'sha256':hashlib.sha256(self.data).hexdigest()}]
        self.config=VMConfig.from_data(self.root,{**json.loads((ROOT/'dev/vm.example.json').read_text()),'guest_build_target':model_seed.TARGET})
        self.mock=patch('aios_dev.model_seed.entries',return_value=self.values);self.mock.start();self.addCleanup(self.mock.stop)
        directory=provision.private_directory(self.root,model_seed.CACHE)
        provision.write_new(directory/'model.gguf',self.data,0o444)
        provision.write_json_new(directory/'receipt.json',{'files':self.values,'model_lock_sha256':provision.digest_file(self.root/'models/lock.json')})

    def test_valid_reads_may_update_atime_without_changing_identity(self):
        file=self.root/model_seed.CACHE/'model.gguf'
        os.utime(file,ns=(1,file.stat().st_mtime_ns))
        self.assertEqual(model_seed.validate_cache(self.root),self.values)

    def test_corruption_and_unexpected_file_are_denied_before_media_creation(self):
        directory=self.root/model_seed.CACHE
        extra=directory/'private.key';extra.write_text('must never be copied')
        with self.assertRaises(DevctlError):model_seed.validate_cache(self.root)
        extra.unlink(); file=directory/'model.gguf';file.chmod(0o600);file.write_bytes(b'wrong bytes');file.chmod(0o444)
        with self.assertRaises(DevctlError):model_seed.validate_cache(self.root)

    def test_symlink_and_writable_data_are_rejected(self):
        file=self.root/model_seed.CACHE/'model.gguf';file.chmod(0o600)
        with self.assertRaises(DevctlError):model_seed.validate_cache(self.root)
        file.unlink();file.symlink_to('/etc/passwd')
        with self.assertRaises(DevctlError):model_seed.validate_cache(self.root)

    def test_seed_is_independent_copy_and_installer_rejects_corruption_before_import(self):
        seed=self.root/'seed';seed.mkdir();model_seed.stage(self.config,seed)
        src=self.root/model_seed.CACHE/'model.gguf';dest=seed/'model/model.gguf'
        self.assertNotEqual(src.stat().st_ino,dest.stat().st_ino)
        dest.chmod(0o600);dest.write_bytes(b'wrong bytes')
        binpath=self.root/'bin';binpath.mkdir()
        marker=self.root/'imported'
        nix=binpath/'nix-store';nix.write_text('#!/bin/sh\ntouch "'+str(marker)+'"\n');nix.chmod(0o700)
        result=subprocess.run(['bash',str(seed/'model-import.sh'),'--import'],env={**os.environ,'PATH':str(binpath)+':'+os.environ['PATH']},capture_output=True)
        self.assertNotEqual(result.returncode,0);self.assertFalse(marker.exists())

    def test_model_disabled_seed_does_not_transfer_data(self):
        seed=self.root/'seed';seed.mkdir()
        config=VMConfig.from_data(self.root,{**self.config.values,'guest_build_target':'aios-dev'})
        model_seed.stage(config,seed);self.assertEqual(list(seed.iterdir()),[])

    def test_arbitrary_exports_fail_without_ssh(self):
        with patch('aios_dev.model_seed.guest.enrolled_identity') as enroll:
            for value in ['/etc/shadow','/nix/store/../../home/dev/private','https://example.com/model']:
                with self.assertRaises(DevctlError):model_seed.export(self.config,value)
            enroll.assert_not_called()

if __name__=='__main__':unittest.main()
