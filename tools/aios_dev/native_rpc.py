"""Fixed installed-service probes with a live SSH caller, no shell API."""
from datetime import datetime, timezone
from pathlib import Path
import shlex
import uuid

from . import acceptance, guest, provision, sync
from .config import invalid
from .errors import ExitCode


def run(config):
    return _run(config, journal=False)


def run_journal(config):
    return _run(config, journal=True)


def run_process(config):
    return _run(config, journal=False, process=True)


def _run(config, *, journal, process=False):
    if not config.root.is_relative_to(acceptance.STORAGE_ROOT):
        raise invalid("Installed native qualification requires storage under /mnt/Storage")
    trust, identity = guest.enrolled_identity(config)
    _, publication = sync.synchronize(config)
    if guest.enrolled_identity(config)[1] != identity:
        raise provision.failure(ExitCode.TARGET_MISMATCH, "EXECUTOR_TARGET_CHANGED", "Target changed after source publication")
    provenance = sync.contract.decode(Path(publication["artifact_path"]).read_bytes())
    source = Path(publication["guest_source_path"])
    expected = Path(config.values["guest_source_root"]) / publication["release_digest"]
    if source != expected or len(publication["release_digest"]) != 64:
        raise invalid("Installed native probe requires the verified immutable source")
    script = source / ("tools/guest/installed_process_smoke.py" if process else "tools/guest/installed_journal_smoke.py" if journal else "tools/guest/installed_executor_smoke.py")
    command = "/run/current-system/sw/bin/python3 " + shlex.quote(str(script))
    started = datetime.now(timezone.utc).isoformat()
    # This fixed foreground operation preserves the SSH PAM/logind session.
    # A detached worker is not an authenticated live originating client.
    status, output, errors = sync.exchange([*guest.ssh_arguments(config)[:-1], command], [b""],
        response_limit=2 * 1024 * 1024, timeout=660)
    if guest.enrolled_identity(config)[1] != identity:
        raise provision.failure(ExitCode.TARGET_MISMATCH, "EXECUTOR_TARGET_CHANGED", "Target changed during installed Executor probe")
    directory = provision.private_directory(config.root, ".local/reports/" + str(uuid.uuid4()))
    log = acceptance.sanitize(output.decode(errors="replace") + errors.decode(errors="replace"))
    prefix = b"AIOS_INSTALLED_PROCESS=" if process else b"AIOS_INSTALLED_JOURNAL=" if journal else b"AIOS_INSTALLED_EXECUTOR "
    records = [line[len(prefix):] for line in output.splitlines() if line.startswith(prefix)]
    observation = None
    try:
        if len(records) == 1:
            observation = sync.contract.decode(records[0])
        if process:
            valid = (isinstance(observation,dict) and observation.get("evidence_kind")=="real-installed-native-process-broker"
                and type(observation.get("uid")) is int and observation["uid"]>=1000
                and type(observation.get("broker_uid")) is int and observation["broker_uid"]==observation["uid"]
                and type(observation.get("broker_pid")) is int and observation["broker_pid"]>1
                and observation.get("boot_id")==identity["boot_id"]
                and isinstance(observation.get("installed_executable"),str)
                and observation["installed_executable"].startswith("/nix/store/")
                and observation["installed_executable"].endswith("/bin/aios-sessiond")
                and observation.get("termination_performed") is False
                and all(observation.get(key) is True for key in ("managed_service_verified","own_uid_filter",
                    "native_child_identity_verified","metrics_schema_verified","natural_exit_refused","cursor_continuation",
                    "cross_connection_refused","query_drift_refused","claimed_uid_refused","app_filter_refused","expiry_refused")))
        elif journal:
            valid = (isinstance(observation,dict) and observation.get("evidence_kind")=="real-installed-native-journal-observer"
                and type(observation.get("uid")) is int and observation["uid"] >= 1000
                and type(observation.get("observer_uid")) is int and observation["observer_uid"]==0
                and type(observation.get("observer_pid")) is int and observation["observer_pid"]>1
                and observation.get("boot_id")==identity["boot_id"] and observation.get("controlled_messages")==3
                and isinstance(observation.get("historical_boot_id"),str)
                and str(uuid.UUID(observation["historical_boot_id"]))==observation["historical_boot_id"]
                and observation["historical_boot_id"]!=identity["boot_id"]
                and observation.get("historical_controlled_messages")==3
                and all(observation.get(key) is True for key in ("own_uid_filter","system_unit_filter","kernel_source",
                    "time_filter","priority_filter","entry_limit","cursor_continuation","cross_connection_refused",
                    "query_drift_refused","missing_boot_refused","claimed_uid_refused","redaction_before_evidence","evidence_hash_verified","expiry_refused",
                    "historical_boot_filter","batch_evidence_boot_verified")))
        else:
            valid = (isinstance(observation,dict) and type(observation.get("uid")) is int and observation["uid"] >= 1000
            and type(observation.get("root_bus_owner_uid")) is int and observation["root_bus_owner_uid"] == 0 and type(observation.get("root_bus_owner_pid")) is int
            and observation["root_bus_owner_pid"] > 0
            and all(observation.get(key) is True for key in ("native_caller_and_baseline_verified","typed_denials_verified",
                "reconnect_denials_verified","durable_pre_effect_cancellation_verified","bus_ownership_policy_verified","system_and_packages_verified"))
            and observation.get("trusted_confirmation_verified") is False and observation.get("activation_performed") is False)
    except (ValueError, UnicodeError):
        valid = False
    code = ExitCode.SUCCESS if status == 0 and valid else ExitCode.VERIFICATION_FAILURE
    report = {"schema_version":1,"evidence_kind":"real-installed-process-live-SSH-caller" if process else "real-installed-journal-live-SSH-caller" if journal else "real-installed-executor-live-SSH-caller",
        "source":provenance,"target_identity":identity,"host_key_fingerprint":trust["host_key_fingerprint"],
        "subject":config.values["ssh_user"],"argv":["/run/current-system/sw/bin/python3",str(script)],
        "started_at":started,"finished_at":datetime.now(timezone.utc).isoformat(),"upstream_exit":status,
        "exit_status":int(code),"caller_session_held_open":True,"probe_observation_valid":valid,"probe":observation,
        "limitations":(["Actual installed user broker, controlled naturally exiting child, independent native proc/pidfd identity.",
            "Does not qualify graceful termination, original model task grants, cross-UID callers or actual PID reuse."] if process else ["Controlled public messages; actual installed native journal, root service and authenticated user.",
            "Does not qualify model task grants, cross-UID reads, user-unit resolution, rotation or hard native-call interruption."] if journal else
            ["Typed fixture intent; actual native root service, user, bus and ledger.",
            "Does not grant trusted graphical consent or qualify activation."])}
    provision.write_new(directory / "log.txt", log.encode())
    provision.write_json_new(directory / "report.json", report)
    return code, {"artifact_path":str(directory / "report.json"),"release_digest":publication["release_digest"],
        "identity_verified":True,"upstream_exit":status,"subject":config.values["ssh_user"],"caller_session_held_open":True}
