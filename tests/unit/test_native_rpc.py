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

    def test_journal_detach_cannot_lose_its_originating_login(self):
        args = cli.parser().parse_args(["test","--suite","integration","--provider","journal-inspection","--detach"])
        with patch.object(cli,"load_config") as load, patch.object(native_rpc,"run_journal") as run:
            with self.assertRaises(DevctlError) as error:
                cli.dispatch(args)
        self.assertEqual(error.exception.exit_code,ExitCode.INVALID_INPUT)
        load.assert_not_called();run.assert_not_called()

    def test_journal_zero_exit_cannot_hide_skipped_or_wrong_boot_cases(self):
        proof={"evidence_kind":"real-installed-native-journal-observer","uid":1000,"observer_uid":0,"observer_pid":50,
            "boot_id":IDENTITY["boot_id"],"controlled_messages":3,
            "historical_boot_id":"11111111-1111-4111-8111-111111111111","historical_controlled_messages":3}
        for key in ("own_uid_filter","system_unit_filter","kernel_source","time_filter","priority_filter","entry_limit",
            "cursor_continuation","cross_connection_refused","query_drift_refused","missing_boot_refused","claimed_uid_refused",
            "redaction_before_evidence","evidence_hash_verified","expiry_refused","historical_boot_filter","batch_evidence_boot_verified"):proof[key]=True
        incomplete={**proof,"redaction_before_evidence":False}
        expired_unchecked={**proof,"expiry_refused":False}
        expiry_missing={key:value for key,value in proof.items() if key!="expiry_refused"}
        history_missing={key:value for key,value in proof.items() if key!="historical_boot_filter"}
        history_current={**proof,"historical_boot_id":IDENTITY["boot_id"]}
        history_malformed={**proof,"historical_boot_id":"not-a-boot"}
        batch_unchecked={**proof,"batch_evidence_boot_verified":False}
        drifted={**proof,"boot_id":"another-boot"}
        for output,expected in ((b"",ExitCode.VERIFICATION_FAILURE),
            (b"AIOS_INSTALLED_JOURNAL="+json.dumps(incomplete).encode()+b"\n",ExitCode.VERIFICATION_FAILURE),
            (b"AIOS_INSTALLED_JOURNAL="+json.dumps(expired_unchecked).encode()+b"\n",ExitCode.VERIFICATION_FAILURE),
            (b"AIOS_INSTALLED_JOURNAL="+json.dumps(expiry_missing).encode()+b"\n",ExitCode.VERIFICATION_FAILURE),
            (b"AIOS_INSTALLED_JOURNAL="+json.dumps(history_missing).encode()+b"\n",ExitCode.VERIFICATION_FAILURE),
            (b"AIOS_INSTALLED_JOURNAL="+json.dumps(history_current).encode()+b"\n",ExitCode.VERIFICATION_FAILURE),
            (b"AIOS_INSTALLED_JOURNAL="+json.dumps(history_malformed).encode()+b"\n",ExitCode.VERIFICATION_FAILURE),
            (b"AIOS_INSTALLED_JOURNAL="+json.dumps(batch_unchecked).encode()+b"\n",ExitCode.VERIFICATION_FAILURE),
            (b"AIOS_INSTALLED_JOURNAL="+json.dumps(drifted).encode()+b"\n",ExitCode.VERIFICATION_FAILURE),
            (b"AIOS_INSTALLED_JOURNAL="+json.dumps(proof).encode()+b"\n",ExitCode.SUCCESS)):
            with patch.object(native_rpc.acceptance,"STORAGE_ROOT",self.root), \
                 patch.object(native_rpc.guest,"enrolled_identity",return_value=({"host_key_fingerprint":"fixture-pin"},IDENTITY)), \
                 patch.object(native_rpc.sync,"synchronize",return_value=(0,self.publication)), \
                 patch.object(native_rpc.guest,"ssh_arguments",return_value=["ssh","pinned-fixture","identity"]), \
                 patch.object(native_rpc.sync,"exchange",return_value=(0,output,b"")) as exchange:
                code,result=native_rpc.run_journal(self.config)
            self.assertEqual(code,expected)
            self.assertTrue(exchange.call_args.args[0][-1].endswith('/tools/guest/installed_journal_smoke.py'))
            self.assertTrue(json.loads(Path(result['artifact_path']).read_text())['caller_session_held_open'])

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

    def test_process_probe_cannot_detach_or_infer_success_from_zero_exit(self):
        args=cli.parser().parse_args(["test","--suite","integration","--provider","process-inspection","--detach"])
        with patch.object(cli,"load_config") as load, patch.object(native_rpc,"run_process") as run:
            with self.assertRaises(DevctlError):cli.dispatch(args)
        load.assert_not_called();run.assert_not_called()
        proof={"evidence_kind":"real-installed-native-process-broker","uid":1000,"broker_uid":1000,"broker_pid":50,
            "boot_id":IDENTITY["boot_id"],"installed_executable":"/nix/store/fixture-aios-core/bin/aios-sessiond","termination_performed":False,
            "process_provider_uid":1000,"process_provider_pid":51,"process_provider_executable":"/nix/store/fixture-aios-core/bin/aios-processd"}
        flags=("managed_service_verified","own_uid_filter","native_child_identity_verified","metrics_schema_verified",
            "natural_exit_refused","cursor_continuation","cross_connection_refused","query_drift_refused","claimed_uid_refused","app_filter_refused","expiry_refused","process_component_verified",
            "unmanaged_native_caller_refused","unix_origin_forwarding_verified")
        proof.update(dict.fromkeys(flags,True))
        cases=[(b"",ExitCode.VERIFICATION_FAILURE),(proof,ExitCode.SUCCESS),({**proof,"termination_performed":True},ExitCode.VERIFICATION_FAILURE),
            ({**proof,"broker_uid":0},ExitCode.VERIFICATION_FAILURE),({**proof,"boot_id":"drifted"},ExitCode.VERIFICATION_FAILURE),
            ({**proof,"installed_executable":"/tmp/aios-sessiond"},ExitCode.VERIFICATION_FAILURE),
            ({**proof,"process_provider_uid":0},ExitCode.VERIFICATION_FAILURE),
            ({**proof,"process_provider_executable":"/tmp/aios-processd"},ExitCode.VERIFICATION_FAILURE),
            ({k:v for k,v in proof.items() if k!="process_provider_pid"},ExitCode.VERIFICATION_FAILURE)]
        for flag in flags:
            cases.append(({**proof,flag:False},ExitCode.VERIFICATION_FAILURE))
            cases.append(({k:v for k,v in proof.items() if k!=flag},ExitCode.VERIFICATION_FAILURE))
        for value,expected in cases:
            output=b"AIOS_INSTALLED_PROCESS="+json.dumps(value).encode()+b"\n" if isinstance(value,dict) else value
            with patch.object(native_rpc.acceptance,"STORAGE_ROOT",self.root), \
                 patch.object(native_rpc.guest,"enrolled_identity",return_value=({"host_key_fingerprint":"fixture-pin"},IDENTITY)), \
                 patch.object(native_rpc.sync,"synchronize",return_value=(0,self.publication)), \
                 patch.object(native_rpc.guest,"ssh_arguments",return_value=["ssh","pinned-fixture","identity"]), \
                 patch.object(native_rpc.sync,"exchange",return_value=(0,output,b"")) as exchange:
                code,result=native_rpc.run_process(self.config)
            self.assertEqual(code,expected)
            self.assertTrue(exchange.call_args.args[0][-1].endswith('/tools/guest/installed_process_smoke.py'))
            record=json.loads(Path(result['artifact_path']).read_text())
            self.assertTrue(record['caller_session_held_open'])
            self.assertEqual(record['evidence_kind'],'real-installed-process-live-SSH-caller')

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
