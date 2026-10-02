"""Host-owned disposable KVM guests for registered bootstrap guard cases.

No host Nix, nested virtualization, arbitrary guest commands or host mounts.
Synthetic existing filesystems qualify installer denials, not OS acceptance.
"""
from datetime import datetime, timezone
import os
from pathlib import Path
import re
import socket
import time
import uuid

from .config import VMConfig, invalid, read_json
from .errors import DevctlError, ExitCode
from . import provision, sync, vm

CASES = {"wrong-disk": "d", "wrong-dmi": "u", "wrong-authorization": "a", "reinstall": "r"}
WRONG_SERIAL = "AIOS_WRONG_ROOT"
STORAGE_ROOT = Path("/mnt/Storage")


def sanitize(text):
    text = re.sub(r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----.*?(?:-----END [A-Z0-9 ]*PRIVATE KEY-----|$)", "[REDACTED PRIVATE KEY]", text, flags=re.S)
    text = re.sub(r"(?im)(authorization\s*[:=]\s*)[^\n]*", r"\1[REDACTED]", text)
    text = re.sub(r"(?i)((?:password|api[_-]?key|access[_-]?token|secret)\s*[:=]\s*)[^\s,;]+", r"\1[REDACTED]", text)
    text = re.sub(r"\bsk-[A-Za-z0-9_-]{16,}\b", "[REDACTED TOKEN]", text)
    text = re.sub(r"\x1b\[[0-9;?]*[A-Za-z]", "", text)
    return "".join(c for c in text if c in "\n\t" or 32 <= ord(c) != 127)


def validate_disposable(config, record, case):
    from .guest import private_file
    if case not in CASES:
        raise invalid("Unregistered disposable bootstrap case")
    path = config.root / ".local/disposable.json"
    private_file(path)
    value = read_json(path)
    fields = {"schema_version", "owner_workspace", "run_id", "case", "configuration", "guest_uuid", "installation_uuid", "disk_device", "disk_inode", "source_head", "source_dirty", "source_snapshot_digest"}
    if not isinstance(value, dict) or set(value) != fields or type(value["schema_version"]) is not int or value["schema_version"] != 1:
        raise invalid("Invalid disposable fixture binding")
    try:
        run = uuid.UUID(value["run_id"])
        owner = Path(value["owner_workspace"]).resolve(strict=True)
    except (ValueError, OSError, TypeError) as error:
        raise invalid("Invalid disposable workspace identity") from error
    if str(run) != value["run_id"] or str(owner) != value["owner_workspace"] or type(value["source_dirty"]) is not bool or not re.fullmatch(r"[0-9a-f]{40}", str(value["source_head"])) or not re.fullmatch(r"[0-9a-f]{64}", str(value["source_snapshot_digest"])):
        raise invalid("Invalid disposable source provenance")
    expected = owner / ".local/a" / run.hex[:8] / CASES[case]
    if not owner.is_relative_to(STORAGE_ROOT) or config.root != expected or value["case"] != case or value["configuration"] != config.values:
        raise invalid("Bootstrap qualification requires an isolated project-owned disposable workspace")
    plan = record["plan"]
    if any(value[field] != record[field] for field in ("disk_device", "disk_inode")) or any(value[field] != plan[field] for field in ("guest_uuid", "installation_uuid")):
        raise provision.failure(ExitCode.TARGET_MISMATCH, "DISPOSABLE_TARGET_MISMATCH", "Disposable case no longer matches its fresh VM/disk identity")
    return value


def console_command(config, record, case):
    validate_disposable(config, record, case)
    plan = record["plan"]
    serial = WRONG_SERIAL if case == "wrong-disk" else provision.DISK_SERIAL
    script = r'''#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
test "$EUID" = 0
test "$(sed -n 's/^ID=//p' /etc/os-release | tr -d '"')" = nixos
virt=$(systemd-detect-virt --vm)
test "$virt" = kvm
test "$(tr '[:upper:]' '[:lower:]' </sys/class/dmi/id/product_uuid)" = GUEST_UUID
test "$(cat /sys/class/block/vda/serial)" = EXPECTED_SERIAL
test "$(readlink -f /sys/class/block/vda/device/driver)" = /sys/bus/virtio/drivers/virtio_blk
test "$(lsblk --nodeps --noheadings --output NAME,TYPE | awk '$2 == "disk" {print $1}' | xargs)" = vda
test "$(lsblk --list --noheadings --output NAME /dev/vda | wc -l)" = 1
test -z "$(wipefs --no-act --noheadings --output TYPE /dev/vda)"
test -z "$(findmnt --noheadings --raw --source /dev/vda || true)"
mkdir -p /run/aios-seed
mount -o ro /dev/disk/by-label/AIOS_SEED /run/aios-seed
cd /run/aios-seed
sha256sum --check --strict manifest.sha256
test "$(cat guest.uuid)" = GUEST_UUID
test "$(cat authorized.uuid)" = GUEST_UUID
test "$(cat installation.uuid)" = INSTALL_UUID
test "$(cat disk.serial)" = AIOS_DEV_ROOT
printf 'AIOS_QUALIFICATION_TARGET DMI=%s install=%s role=disposable-bootstrap-fixture serial=%s virt=%s
' GUEST_UUID INSTALL_UUID EXPECTED_SERIAL "$virt"
seed=/run/aios-seed
SETUP_CASE
before=$(sha256sum /dev/vda | awk '{print $1}')
set +e
bash "$seed/bootstrap.sh" >/run/aios-denial.log 2>&1
status=$?
set -e
cat /run/aios-denial.log
test "$status" = 4
grep -F -- 'EXPECTED_DENIAL' /run/aios-denial.log
after=$(sha256sum /dev/vda | awk '{print $1}')
test "$before" = "$after"
printf 'AIOS_QUALIFICATION_ASSERT case=%s installer_exit=%s disk_before=%s disk_after=%s unchanged=true
' CASE_NAME "$status" "$before" "$after"
cd /
umount /run/aios-seed
'''
    if case in ("wrong-dmi", "wrong-authorization"):
        field = "guest.uuid" if case == "wrong-dmi" else "authorized.uuid"
        setup = r'''cp -r /run/aios-seed /run/aios-test-seed
seed=/run/aios-test-seed
chmod -R u+w "$seed"
printf '%s\n' 11111111-1111-4111-8111-111111111111 >"$seed/FIELD_NAME"
(cd "$seed"; sha256sum FIELD_NAME > /run/aios-new-hash; sed '/  FIELD_NAME$/d' manifest.sha256 >/run/aios-new-manifest; cat /run/aios-new-hash >>/run/aios-new-manifest; cp /run/aios-new-manifest manifest.sha256)
'''.replace("FIELD_NAME", field)
    elif case == "reinstall":
        setup = r'''# Synthetic installed data only on this fresh authorized disposable disk.
sgdisk --clear --disk-guid=GUEST_UUID --new=1:0:+1G --typecode=1:ef00 --change-name=1:AIOS_DEV_EFI --new=2:0:0 --typecode=2:8300 --change-name=2:AIOS_DEV_ROOT /dev/vda
udevadm settle
mkfs.btrfs -U INSTALL_UUID -L AIOS_DEV_ROOT /dev/vda2
mkdir -p /run/aios-existing
mount /dev/vda2 /run/aios-existing
btrfs subvolume create /run/aios-existing/@root
mkdir -p /run/aios-existing/@root/etc/aios
printf '%s\n' INSTALL_UUID >/run/aios-existing/@root/etc/aios/installation-uuid
printf '%s\n' synthetic-disposable-install >/run/aios-existing/@root/etc/aios/fixture
umount /run/aios-existing
sync
'''
    else:
        setup = ":"
    reasons = {"wrong-disk": "Expected exactly one disk with AIOS_DEV_ROOT serial", "wrong-dmi": "DMI UUID mismatch",
               "wrong-authorization": "No matching host provisioning authorization", "reinstall": "Disk has partitions; reinstall requires separate destructive authorization"}
    script = script.replace("SETUP_CASE", setup).replace("GUEST_UUID", plan["guest_uuid"]).replace("INSTALL_UUID", plan["installation_uuid"])
    script = script.replace("EXPECTED_SERIAL", serial).replace("CASE_NAME", case).replace("EXPECTED_DENIAL", reasons[case])
    return vm.encoded_console_script(script, "AIOS_QUALIFICATION_EXIT")


def free_port():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


def prepare_case(owner, directory, case, run_id, manifest, digest, contents, media):
    if case not in CASES or directory != owner.root / ".local/a" / uuid.UUID(run_id).hex[:8] / CASES[case]:
        raise invalid("Invalid disposable case directory")
    for item, data in zip(manifest["files"], contents):
        target = directory / item["path"]
        target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        provision.write_new(target, data, item["mode"])
    # A local public-only Git fixture satisfies seed tracking. Its HEAD is not
    # represented as the canonical source HEAD in qualification reports.
    provision.run(["git", "-C", str(directory), "init", "-q"])
    provision.run(["git", "-C", str(directory), "add", "."])
    provision.run(["git", "-C", str(directory), "-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false", "-c", "user.name=AIOS Fixture", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "Public disposable source fixture"])
    values = {**owner.values, "ssh_host": "127.0.0.1", "ssh_port": free_port(), "vcpus": 2, "memory_mib": 2048, "disk_gib": 8}
    for field in ("guest_uuid", "installation_uuid", "guest_role"):
        values.pop(field, None)
    config = VMConfig.from_data(directory, values, configured=True)
    provision.private_directory(directory, ".local/vm")
    provision.write_json_new(directory / ".local/vm.json", values)
    # Only immutable, verified installer media are shared; never live disks.
    cached = directory / ".local/vm" / Path(media["path"]).name
    if provision.digest_file(Path(media["path"])) != media["sha256"]:
        raise invalid("Shared official installer cache digest changed")
    os.link(media["path"], cached)
    plan = provision.prepare_plan(config)
    code, result = provision.create(config, plan["guest_uuid"])
    if code != ExitCode.SUCCESS:
        return code, result, config
    record = vm.load_record(config)
    provision.write_json_new(directory / ".local/disposable.json", {
        "schema_version": 1, "owner_workspace": str(owner.root), "run_id": run_id, "case": case, "configuration": values,
        "guest_uuid": plan["guest_uuid"], "installation_uuid": plan["installation_uuid"],
        "disk_device": record["disk_device"], "disk_inode": record["disk_inode"], "source_head": manifest["git_head"],
        "source_dirty": manifest["dirty"], "source_snapshot_digest": digest,
    })
    # No private key/secret file is allowed in the seed. Source collection has
    # already rejected private key headers and excluded private artifact paths.
    seed = directory / ".local/vm/seed"
    for path in seed.rglob("*"):
        if path.is_file() and (path.suffix in sync.contract.PRIVATE_SUFFIXES or sync.contract.KEY_HEADER.search(path.read_bytes())):
            raise invalid("Credential or private artifact entered disposable seed")
    return code, result, config


def run_bootstrap_guards(owner, selected=None):
    from .guest import enrolled_identity
    if owner.values["provider"] != "qemu" or not owner.root.is_relative_to(STORAGE_ROOT):
        raise invalid("Disposable KVM tests require a managed workspace under /mnt/Storage")
    cases = [selected] if selected else list(CASES)
    if any(case not in CASES for case in cases):
        raise invalid("Unknown bootstrap qualification case")
    trust, identity = enrolled_identity(owner)
    cache = provision.private_directory(owner.root, ".local/installer-cache")
    media = provision.fetch_media(cache)
    manifest, digest, contents = sync.collect(owner.root)
    run_id = str(uuid.uuid4())
    base = provision.private_directory(owner.root, ".local/a/" + uuid.UUID(run_id).hex[:8])
    reports = provision.private_directory(owner.root, ".local/reports/" + run_id)
    results, code = [], ExitCode.SUCCESS
    for case in cases:
        directory = provision.private_directory(owner.root, str((base / CASES[case]).relative_to(owner.root)))
        result = {"case": case, "workspace": str(directory), "evidence_kind": "real-KVM-installer-guard-with-disposable-fixture"}
        config = None
        try:
            code, prepared, config = prepare_case(owner, directory, case, run_id, manifest, digest, contents, media)
            result["prepared"] = prepared
            if code == ExitCode.SUCCESS:
                record = vm.load_record(config)
                result["target"] = {"guest_uuid": record["plan"]["guest_uuid"], "installation_uuid": record["plan"]["installation_uuid"],
                                    "role": "disposable-bootstrap-fixture", "media": record["media"], "seed_sha256": record["seed_sha256"],
                                    "firmware_sha256": record["firmware_sha256"], "disk_device": record["disk_device"], "disk_inode": record["disk_inode"]}
                with provision.operation_lock(config.root):
                    _, result["started"] = vm.start(config, "none", True, qualification=case)
                result["qemu_process"] = read_json(config.root / ".local/vm/process.json")
                # Let the official ISO reach its console. The registered action
                # requires a bounded serial handshake and retains failed evidence.
                deadline = time.monotonic() + 35
                while time.monotonic() < deadline:
                    vm.verify_process(result["qemu_process"])
                    time.sleep(1)
                _, result["console_before"] = vm.console(config, capture=True)
                with provision.operation_lock(config.root):
                    code, observation = vm.console(config, qualification=case)
                result["qualification"] = observation
                log = Path(observation["artifact_path"])
                result["console_sha256"] = provision.digest_file(log)
                if code == ExitCode.SUCCESS:
                    data = log.read_text()
                    match = re.search(r"AIOS_QUALIFICATION_ASSERT case=([a-z-]+) installer_exit=4 disk_before=([0-9a-f]{64}) disk_after=([0-9a-f]{64}) unchanged=true", data)
                    target = result["target"]
                    serial = WRONG_SERIAL if case == "wrong-disk" else provision.DISK_SERIAL
                    expected = "AIOS_QUALIFICATION_TARGET DMI=" + target["guest_uuid"] + " install=" + target["installation_uuid"] + " role=disposable-bootstrap-fixture serial=" + serial + " virt=kvm"
                    if not match or match[1] != case or match[2] != match[3] or expected not in data:
                        code = ExitCode.VERIFICATION_FAILURE
                    else:
                        result["assertions"] = {"installer_denied": True, "full_virtual_disk_byte_digest_unchanged": True, "disk_sha256": match[2], "seed_public_only": True, "guest_preflight_identity_verified": True}
        except DevctlError as error:
            code = error.exit_code
            result["error"] = {"code": error.code, "message": str(error), **error.details}
        except (OSError, ValueError) as error:
            code = ExitCode.OPERATION_FAILURE
            result["error"] = {"code": "OPERATION_FAILED", "message": type(error).__name__}
        finally:
            if config is not None and (config.root / ".local/vm/process.json").exists():
                try:
                    with provision.operation_lock(config.root):
                        _, result["stopped"] = vm.stop(config)
                except DevctlError as error:
                    result["stop_error"] = {"code": error.code, "message": str(error)}
                    if code == ExitCode.SUCCESS:
                        code = error.exit_code
            result["exit_status"] = int(code)
        results.append(result)
        result["console_evidence"] = []
        for log in sorted(directory.glob(".local/vm/console/qualification-*.log")):
            with log.open("rb") as handle:
                handle.seek(max(0, log.stat().st_size - 65536))
                tail = sanitize(handle.read(65536).decode(errors="replace"))
            result["console_evidence"].append({"path": str(log), "sha256": provision.digest_file(log), "sanitized_tail": tail, "tail_limit_bytes": 65536})
            provision.write_new(reports / (case + ".log"), tail.encode())
        if code != ExitCode.SUCCESS:
            break
    report = {"schema_version": 1, "run_id": run_id, "suite": "bootstrap-guards", "observed_at": datetime.now(timezone.utc).isoformat(),
              "command": ["python3", "tools/devctl.py", "test", "--suite", "integration", "--bootstrap-case", selected or "all", "--json"],
              "development_guest_identity": identity, "development_guest_host_key_fingerprint": trust["host_key_fingerprint"], "requested_cases": cases,
              "source_head": manifest["git_head"], "source_dirty": manifest["dirty"], "snapshot_digest": digest,
              "lock_hashes": {item["path"]: item["sha256"] for item in manifest["files"] if item["path"] in {"flake.lock", "Cargo.lock"}},
              "model_hash": None, "runtime_hash": None, "cases": results, "exit_status": int(code),
              "limitations": ["Real host-launched KVM installer/guard checks with synthetic existing filesystem data; not final OS, desktop, provider or model acceptance.",
                              "Installer trust uses verified official ISO/seed hashes and pinned QMP process/peer/UUID/disk plus in-guest preflight; installer SSH enrollment is not claimed."]}
    path = reports / "report.json"
    provision.write_json_new(path, report)
    provision.write_new(reports / "summary.txt", ("Bootstrap guard qualification: " + str(int(code)) + "\n" + "\n".join(case["case"] + ": " + str(case["exit_status"]) for case in results) + "\n").encode())
    return code, {"run_id": run_id, "suite": "bootstrap-guards", "artifact_path": str(path), "release_digest": digest,
                  "case_count": len(results), "evidence_kind": results[0]["evidence_kind"], "identity_verified": False}
