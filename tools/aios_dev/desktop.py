"""Host-controlled synthetic disposable Plasma qualification through SSH/QMP.

Only the registered desktop image and probe are available. Failed installations
are retained for inspection; a resume never formats or restarts an attempted job.
"""
from datetime import datetime, timezone
import base64
import os
import re
from pathlib import Path
import time
import uuid

from . import acceptance, guest, provision, sync, vm, model_seed
from .config import VMConfig, invalid, load_config, read_json
from .errors import DevctlError, ExitCode

PROFILE = "synthetic-disposable-plasma-wayland-v1"
PROBE = Path(__file__).resolve().parents[1] / "guest/desktop_probe.py"


def read_binding_record(path):
    # The binding includes a source manifest, whose bound differs from the
    # small VM configuration. Keep the configuration reader's 64 KiB limit.
    guest.private_file(path)
    limit = sync.contract.MAX_HEADER + 4096
    if path.stat().st_size > limit:
        raise invalid("Desktop source binding exceeds its manifest limit")
    with path.open("rb") as handle:
        data = handle.read(limit + 1)
    if len(data) > limit:
        raise invalid("Desktop source binding exceeds its manifest limit")
    try:
        return sync.contract.decode(data)
    except (ValueError, UnicodeError) as error:
        raise invalid("Invalid desktop source binding JSON") from error


def identifier(value):
    try:
        parsed = uuid.UUID(value)
        if str(parsed) != value:
            raise ValueError
    except (TypeError, ValueError, AttributeError) as error:
        raise invalid("Desktop run must be a canonical UUID") from error
    return parsed


def binding(owner, run_id):
    directory = owner.root / ".local/d" / identifier(run_id).hex[:8]
    config = load_config(directory)
    path = directory / ".local/desktop.json"
    value = read_binding_record(path)
    if set(value) != {"schema_version", "run_id", "owner_workspace", "configuration", "guest_uuid", "installation_uuid", "disk_device", "disk_inode", "manifest", "snapshot_digest"} or value["schema_version"] != 1 or value["run_id"] != run_id or value["owner_workspace"] != str(owner.root) or value["configuration"] != config.values:
        raise invalid("Desktop fixture binding mismatch")
    if not owner.root.is_relative_to(acceptance.STORAGE_ROOT) or directory.resolve() != directory or config.values["guest_build_target"] not in ("aios-desktop-test", model_seed.TARGET):
        raise invalid("Desktop runner requires its isolated workspace under /mnt/Storage")
    if sync.contract.validate_manifest(value["manifest"]) != value["snapshot_digest"]:
        raise invalid("Desktop source manifest mismatch")
    actual, _, _ = sync.collect(directory)
    if actual["files"] != value["manifest"]["files"]:
        raise provision.failure(ExitCode.TARGET_MISMATCH, "DESKTOP_SOURCE_CHANGED", "Disposable image source changed after registration")
    record = vm.load_record(config)
    if any(value[field] != record[field] for field in ("disk_device", "disk_inode")) or any(value[field] != record["plan"][field] for field in ("guest_uuid", "installation_uuid")):
        raise provision.failure(ExitCode.TARGET_MISMATCH, "DESKTOP_TARGET_CHANGED", "Disposable VM/disk identity changed")
    return config, value


def prepare(owner, *, with_model=False):
    if owner.values["provider"] != "qemu" or not owner.root.is_relative_to(acceptance.STORAGE_ROOT):
        raise invalid("Desktop runner requires a managed owner under /mnt/Storage")
    # Verify the developer target before collecting any source or creating a VM.
    guest.enrolled_identity(owner)
    if with_model:
        model_seed.validate_cache(owner.root)
    manifest, digest, contents = sync.collect(owner.root)
    run_id = str(uuid.uuid4())
    directory = provision.private_directory(owner.root, ".local/d/" + identifier(run_id).hex[:8])
    for item, data in zip(manifest["files"], contents):
        target = directory / item["path"]
        target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        provision.write_new(target, data, item["mode"])
    provision.run(["git", "-C", str(directory), "init", "-q"])
    provision.run(["git", "-C", str(directory), "add", "."])
    provision.run(["git", "-C", str(directory), "-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false", "-c", "user.name=AIOS Fixture", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "Public synthetic desktop fixture"])
    values = {**owner.values, "guest_build_target": model_seed.TARGET if with_model else "aios-desktop-test", "ssh_host": "127.0.0.1", "ssh_port": acceptance.free_port(),
              "vcpus": 8, "memory_mib": 16384, "disk_gib": 96}
    for field in ("guest_uuid", "installation_uuid", "guest_role"):
        values.pop(field, None)
    config = VMConfig.from_data(directory, values, configured=True)
    provision.private_directory(directory, ".local/vm")
    provision.write_json_new(directory / ".local/vm.json", values)
    media = provision.fetch_media(provision.private_directory(owner.root, ".local/installer-cache"))
    if provision.digest_file(Path(media["path"])) != media["sha256"]:
        raise invalid("Installer cache changed")
    os.link(media["path"], directory / ".local/vm" / Path(media["path"]).name)
    if with_model:
        model_seed.copy_cache(owner.root, directory)
    plan = provision.prepare_plan(config)
    code, result = provision.create(config, plan["guest_uuid"])
    if code != ExitCode.SUCCESS:
        raise DevctlError(code, "DESKTOP_PROVISION_FAILED", "Disposable desktop provisioning failed")
    record = vm.load_record(config)
    provision.write_json_new(directory / ".local/desktop.json", {
        "schema_version": 1, "run_id": run_id, "owner_workspace": str(owner.root), "configuration": values,
        "guest_uuid": plan["guest_uuid"], "installation_uuid": plan["installation_uuid"],
        "disk_device": record["disk_device"], "disk_inode": record["disk_inode"], "manifest": manifest, "snapshot_digest": digest,
    })
    return run_id


def observe(config):
    trust, identity = guest.enrolled_identity(config)
    # Like the registered job controller, only fixed public probe source enters
    # this unprivileged SSH operation. No caller script/argument is accepted.
    encoded = base64.b64encode(PROBE.read_bytes()).decode()
    command = '/run/current-system/sw/bin/python3 -I -c "import base64;exec(compile(base64.b64decode(\'' + encoded + '\'),\'<aios-desktop-probe>\',\'exec\'))"'
    status, output, _ = sync.exchange([*guest.ssh_arguments(config)[:-1], command], [b""], response_limit=65536, timeout=20)
    if status:
        raise DevctlError(ExitCode.UNMET_PREREQUISITE, "DESKTOP_NOT_READY", "Synthetic desktop session is not ready", details={"upstream_exit": status})
    try:
        value = sync.contract.decode(output)
        valid = (set(value) == {"schema_version", "profile", "boot_id", "tester_uid", "session_id", "processes"}
                 and type(value["schema_version"]) is int and value["schema_version"] == 1 and value["profile"] == PROFILE
                 and value["boot_id"] == identity["boot_id"] and type(value["tester_uid"]) is int and value["tester_uid"] > 0
                 and isinstance(value["session_id"], str) and 0 < len(value["session_id"]) <= 64
                 and isinstance(value["processes"], list) and 2 <= len(value["processes"]) <= 8
                 and {item["name"] for item in value["processes"]} == {"kwin_wayland", "plasmashell"}
                 and all(set(item) == {"pid", "uid", "name", "executable"} and type(item["pid"]) is int and item["pid"] > 0 and type(item["uid"]) is int and item["uid"] == value["tester_uid"]
                         and isinstance(item["executable"], str) and re.fullmatch(r"/nix/store/[a-z0-9]{32}-[A-Za-z0-9.+_-]+/bin/" + item["name"], item["executable"]) for item in value["processes"]))
    except (ValueError, TypeError, KeyError, UnicodeError, StopIteration):
        valid = False
    if not valid:
        raise provision.failure(ExitCode.VERIFICATION_FAILURE, "DESKTOP_PROBE_MISMATCH", "Desktop probe does not match this live synthetic session")
    observation = {"identity": identity, "host_key_fingerprint": trust["host_key_fingerprint"], "desktop": value}
    if config.values["guest_build_target"] == model_seed.TARGET:
        guest.enrolled_identity(config)
        code, output, _ = sync.exchange([*guest.ssh_arguments(config)[:-1], "/run/current-system/sw/bin/installed_model_lifecycle_smoke"],
                                       [b""], response_limit=131072, timeout=20)
        if guest.enrolled_identity(config)[1] != identity:
            raise invalid("Model lifecycle target changed during observation")
        if code == 3:
            pending = sync.contract.decode(output)
            phase = pending.get("phase")
            details = {}
            if phase is not None:
                if (pending.get("state") != "pending" or pending.get("identity") != identity or not isinstance(phase, dict)
                        or phase.get("identity") != identity or phase.get("evidence_kind") != "actual-fixed-model-failure-phase"):
                    raise invalid("Pending model failure phase belongs to another target")
                current = phase.get("current_model_unit", {})
                outage = (phase.get("phase") == "crash_outage" and current.get("ActiveState") in ("activating", "inactive", "failed", "deactivating") and current.get("MainPID") == "0")
                corrupt = (phase.get("phase") == "corrupt_model" and current.get("ActiveState") == "active"
                           and current.get("MainPID") == phase.get("model_unit", {}).get("MainPID"))
                if outage or corrupt:
                    details = {"pending_observation": {**observation, "model_failure_phase": phase,
                               "host_observed_at": datetime.now(timezone.utc).isoformat(), "pinned_ssh_commands_succeeded": True}}
            raise DevctlError(ExitCode.UNMET_PREREQUISITE, "MODEL_LIFECYCLE_PENDING", "The same fixed initial model lifecycle test is still running", details=details)
        if code != 0:
            raise DevctlError(ExitCode.VERIFICATION_FAILURE, "MODEL_LIFECYCLE_FAILED", "Fixed model lifecycle test failed; disposable VM retained", details={"upstream_exit": code})
        proof = sync.contract.decode(output)
        if proof.get("state") != "verified" or proof.get("proof", {}).get("identity") != identity or proof["proof"].get("verified") is not True:
            raise invalid("Installed model lifecycle report does not match this live guest")
        observation["model_lifecycle"] = proof
    return observation


def run(owner, run_id=None, *, with_model=False):
    if run_id is None:
        run_id = prepare(owner, with_model=with_model)
    config, value = binding(owner, run_id)
    record = vm.load_record(config)
    host_manifest, host_digest, _ = sync.collect(owner.root)
    reports = provision.private_directory(owner.root, ".local/reports/" + run_id)
    report = {"schema_version": 1, "run_id": run_id, "suite": "desktop", "evidence_kind": "real-host-KVM-synthetic-Plasma-session",
              "source_head": value["manifest"]["git_head"], "source_dirty": value["manifest"]["dirty"], "snapshot_digest": value["snapshot_digest"],
              "host_source": {"manifest": host_manifest, "snapshot_digest": host_digest},
              "lock_hashes": {item["path"]: item["sha256"] for item in value["manifest"]["files"] if item["path"] in {"flake.lock", "Cargo.lock"}},
              "model_hash": None, "runtime_hash": None, "workspace": str(config.root), "steps": [], "model_failure_observations": [],
              "target": {"guest_uuid": value["guest_uuid"], "installation_uuid": value["installation_uuid"], "guest_role": "development",
                         "fixture_role": "synthetic-disposable-desktop", "disk_serial": provision.DISK_SERIAL,
                         "media_sha256": record["media"]["sha256"], "seed_sha256": record["seed_sha256"], "firmware_sha256": record["firmware_sha256"]},
              "limitations": ["Synthetic tester autologin is exclusive to the disposable test image; production autologin remains forbidden.",
                              "Qualifies host SSH/QMP desktop orchestration, not product UI/app/model acceptance."]}
    def step(name, action):
        code, result = action()
        report["steps"].append({"operation": name, "exit_status": int(code), "result": result})
        if code != ExitCode.SUCCESS:
            raise DevctlError(code, "DESKTOP_STEP_FAILED", "Desktop operation failed: " + name)
        return result
    code = ExitCode.SUCCESS
    try:
        with provision.operation_lock(config.root):
            if not (config.root / ".local/vm/bootstrap-result.json").exists():
                attempted = list(config.root.glob(".local/vm/bootstrap-console-*.json"))
                if attempted:
                    raise provision.failure(ExitCode.UNMET_PREREQUISITE, "DESKTOP_INSTALLATION_INCOMPLETE", "Inspect the retained attempted installation; the desktop runner will not format or retry it")
                if not (config.root / ".local/vm/process.json").exists():
                    step("vm start --bootstrap --display none", lambda: vm.start(config, "none", True))
                time.sleep(35)
                step("vm console --bootstrap-run", lambda: vm.console(config, bootstrap_run=True))
            guest.pin_console(config)
            if not (config.root / ".local/vm/bootstrap-finish.json").exists():
                if list(config.root.glob(".local/vm/finish-console-*.json")):
                    raise provision.failure(ExitCode.UNMET_PREREQUISITE, "DESKTOP_SETUP_INCOMPLETE", "Inspect retained setup evidence before resuming")
                step("vm console --bootstrap-finish", lambda: vm.console(config, bootstrap_finish=True))
            process_path = config.root / ".local/vm/process.json"
            if process_path.exists() and any("id=installer,media=cdrom" in argument for argument in read_json(process_path)["arguments"]):
                step("vm stop --graceful (installer)", lambda: vm.stop(config, graceful=True))
            if not (config.root / ".local/vm/process.json").exists():
                step("vm start --display none (installed)", lambda: vm.start(config, "none", False))
            deadline = time.monotonic() + 180
            while True:
                try:
                    step("enroll", lambda: guest.enroll(config))
                    observation = observe(config)
                    break
                except DevctlError as error:
                    pending = error.details.get("pending_observation")
                    if pending is not None:
                        if len(report["model_failure_observations"]) >= 100:
                            raise invalid("Model failure observation count exceeds bound")
                        report["model_failure_observations"].append(pending)
                    if error.exit_code != ExitCode.UNMET_PREREQUISITE or time.monotonic() >= deadline:
                        raise
                    time.sleep(0.2 if error.code == "MODEL_LIFECYCLE_PENDING" else 3)
            if config.values["guest_build_target"] == model_seed.TARGET:
                phases = {item["model_failure_phase"]["phase"] for item in report["model_failure_observations"]}
                if not {"crash_outage", "corrupt_model"}.issubset(phases):
                    raise DevctlError(ExitCode.VERIFICATION_FAILURE, "MODEL_FAILURE_SSH_UNQUALIFIED", "Actual pinned SSH reads were not observed in both model failure phases")
            report["observation"] = observation
            guest.enrolled_identity(config)
            screenshot = step("vm console --capture", lambda: vm.console(config, capture=True))
            report["screenshot_sha256"] = provision.digest_file(Path(screenshot["artifact_path"]))
            guest.enrolled_identity(config)
            step("vm stop (owned disposable)", lambda: vm.stop(config))
            report["assertions"] = {"active_local_tester_wayland_session": True, "kwin_and_plasmashell_same_uid": True,
                                    "pinned_ssh_identity": True, "qmp_capture_and_owned_shutdown": True, "host_launched_kvm": True}
    except DevctlError as error:
        code = error.exit_code
        report["error"] = {"code": error.code, "message": str(error), **error.details}
    except (OSError, ValueError) as error:
        code = ExitCode.OPERATION_FAILURE
        report["error"] = {"code": "DESKTOP_OPERATION_FAILED", "message": type(error).__name__}
    report["exit_status"] = int(code)
    report["observed_at"] = datetime.now(timezone.utc).isoformat()
    path = reports / (str(uuid.uuid4()) + ".json")
    provision.write_json_new(path, report)
    provision.write_new(path.with_suffix(".txt"), ("Synthetic desktop runner: " + str(int(code)) + "\nRun: " + run_id + "\n" + "\n".join(report["limitations"]) + "\n").encode())
    return code, {"run_id": run_id, "suite": "desktop", "workspace": str(config.root), "artifact_path": str(path),
                  "release_digest": value["snapshot_digest"], "identity_verified": code == ExitCode.SUCCESS}
