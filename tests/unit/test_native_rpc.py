import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from test_host_tools import EXAMPLE
from test_guest import IDENTITY
from aios_dev import cli, native_rpc
from aios_dev.config import VMConfig
from aios_dev.errors import DevctlError, ExitCode


class NativeRpcTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="aios-native-rpc-")
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.root.chmod(0o700)
        self.config = VMConfig.from_data(self.root, EXAMPLE)
        self.provenance = self.root / "source.json"
        self.provenance.write_text('{}')
        self.publication = {"guest_source_path":EXAMPLE["guest_source_root"] + "/" + "a"*64,
            "release_digest":"a"*64,"artifact_path":str(self.provenance)}
        self.proof = {"uid":1000,"root_bus_owner_uid":0,"root_bus_owner_pid":50,
            "native_caller_and_baseline_verified":True,"typed_denials_verified":True,"reconnect_denials_verified":True,
            "durable_pre_effect_cancellation_verified":True,"bus_ownership_policy_verified":True,"system_and_packages_verified":True,
            "trusted_confirmation_verified":False,"activation_performed":False}

    def test_detach_is_rejected_before_loading_target_or_starting_rpc(self):
        args = cli.parser().parse_args(["test","--suite","integration","--provider","installed-executor","--detach"])
        with patch.object(cli,"load_config") as load, patch.object(native_rpc,"run") as run:
            with self.assertRaises(DevctlError) as error:
                cli.dispatch(args)
        self.assertEqual(error.exception.exit_code,ExitCode.INVALID_INPUT)
        load.assert_not_called(); run.assert_not_called()

    def test_target_drift_after_publication_stops_before_rpc(self):
        with patch.object(native_rpc.acceptance,"STORAGE_ROOT",self.root), \
             patch.object(native_rpc.guest,"enrolled_identity",side_effect=[({},IDENTITY),({}, {**IDENTITY,"boot_id":"changed"})]), \
             patch.object(native_rpc.sync,"synchronize",return_value=(0,self.publication)), \
             patch.object(native_rpc.sync,"exchange") as exchange:
            with self.assertRaises(DevctlError) as error:
                native_rpc.run(self.config)
        self.assertEqual(error.exception.exit_code,ExitCode.TARGET_MISMATCH)
        exchange.assert_not_called()
        self.assertFalse((self.root/".local/reports").exists())

    def test_zero_exit_requires_positive_native_observations_and_retains_sanitized_evidence(self):
        for output, expected in ((b"",ExitCode.VERIFICATION_FAILURE),
            (b"AIOS_INSTALLED_EXECUTOR "+json.dumps(self.proof).encode()+b"\npassword=fixture-secret\n",ExitCode.SUCCESS)):
            with patch.object(native_rpc.acceptance,"STORAGE_ROOT",self.root), \
                 patch.object(native_rpc.guest,"enrolled_identity",return_value=({"host_key_fingerprint":"fixture-pin"},IDENTITY)), \
                 patch.object(native_rpc.sync,"synchronize",return_value=(0,self.publication)), \
                 patch.object(native_rpc.guest,"ssh_arguments",return_value=["ssh","pinned-fixture","identity"]), \
                 patch.object(native_rpc.sync,"exchange",return_value=(0,output,b"")) as exchange:
                code,result=native_rpc.run(self.config)
            self.assertEqual(code,expected)
            argv=exchange.call_args.args[0]
            self.assertEqual(argv[:2],["ssh","pinned-fixture"])
            self.assertTrue(argv[-1].endswith('/tools/guest/installed_executor_smoke.py'))
            record=json.loads(Path(result["artifact_path"]).read_text())
            self.assertEqual(record["target_identity"],IDENTITY)
            self.assertNotIn("fixture-secret",Path(result["artifact_path"]).with_name("log.txt").read_text())

    def test_non_storage_workspace_stops_before_guest_operations(self):
        with patch.object(native_rpc.acceptance,"STORAGE_ROOT",self.root/"another"), \
             patch.object(native_rpc.guest,"enrolled_identity") as authenticate:
            with self.assertRaises(DevctlError): native_rpc.run(self.config)
        authenticate.assert_not_called()
