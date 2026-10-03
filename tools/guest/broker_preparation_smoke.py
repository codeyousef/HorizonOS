#!/usr/bin/env python3
"""Actual broker package denials and Rust filesystem/SQLite fixture verification.

No root runtime, worker, approval, activation or root candidate publication occurs.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET
import snapshot as source


def main():
    release = Path(__file__).resolve().parents[2]
    reference = "path:" + str(release)
    locked = ["--no-update-lock-file", "--no-write-lock-file"]
    identity = source.identity()
    environment = {k: os.environ[k] for k in ("HOME", "PATH", "TMPDIR", "XDG_CACHE_HOME", "CARGO_HOME", "CARGO_TARGET_DIR") if k in os.environ}
    environment.update(LANG="C.UTF-8", NIX_USER_CONF_FILES="/dev/null", NIX_REMOTE="daemon")
    commands = []

    def run(argv, payload=None, expected=0, timeout=180, extra=None):
        if source.identity() != identity:
            raise RuntimeError("broker check target changed")
        result = subprocess.run(argv, input=payload, capture_output=True, env={**environment, **(extra or {})}, timeout=timeout)
        commands.append({"argv": argv, "upstream_exit": result.returncode,
                         "stdout": result.stdout.decode(errors="replace"), "stderr": result.stderr.decode(errors="replace")})
        if result.returncode != expected:
            print("AIOS_BROKER_PREPARATION_FAILURE " + json.dumps(commands[-1], sort_keys=True), flush=True)
            raise RuntimeError("broker subprocess returned unexpected exit")
        return result.stdout

    formatted = []
    relatives = ("crates/aios-exec/src/lib.rs", "crates/aios-exec/src/main.rs",
        "crates/aios-exec/src/candidate.rs", "crates/aios-exec/src/ledger.rs",
        "crates/aios-exec/src/candidate/tests.rs", "crates/aios-exec/src/ledger/tests.rs",
        "crates/aios-state/src/lib.rs", "crates/aios-exec/src/native.rs", "crates/aios-exec/src/native/tests.rs",
        "crates/aios-exec/src/caller.rs", "crates/aios-exec/src/caller/tests.rs",
        "crates/aios-exec/src/approval.rs", "crates/aios-exec/src/approval/policy.rs",
        "crates/aios-exec/src/approval/polkit.rs", "crates/aios-exec/src/approval/tests.rs")
    for relative in relatives:
        data = (release / relative).read_bytes()
        out = run(["nix", "develop", *locked, reference, "--command", "rustfmt", "--edition", "2024", "--emit", "stdout", "--config", "skip_children=true"], data)
        formatted.append({"path": relative, "source_sha256": hashlib.sha256(data).hexdigest(),
                          "formatted_sha256": hashlib.sha256(out).hexdigest(), "formatted_source": out.decode()})
    print("AIOS_BROKER_PREPARATION_FORMAT " + json.dumps(formatted, sort_keys=True), flush=True)
    unit = run(["nix", "develop", *locked, reference, "--command", "cargo", "test", "--locked", "-p", "aios-exec", "--lib", "--", "--nocapture"], timeout=180)
    if b"57 passed; 0 failed" not in unit:
        raise RuntimeError("broker fixture test count changed")
    observations = [json.loads(line.split("AIOS_BROKER_CALLER_OBSERVATIONS ", 1)[1]) for line in unit.decode().splitlines() if "AIOS_BROKER_CALLER_OBSERVATIONS " in line]
    if len(observations) != 1 or observations[0]["uid"] != os.getuid() or observations[0]["boot_id"] != identity["boot_id"] or observations[0]["logind_uid"] != 0:
        raise RuntimeError("native read-only caller observations missing or mismatched")
    polkit = [json.loads(line.split("AIOS_POLKIT_OBSERVATIONS ", 1)[1]) for line in unit.decode().splitlines() if "AIOS_POLKIT_OBSERVATIONS " in line]
    if len(polkit) != 1 or polkit[0]["boot_id"] != identity["boot_id"] or polkit[0]["check_authorization_called"]:
        raise RuntimeError("read-only polkit observations missing or called authentication")
    outputs = json.loads(run(["nix", "build", "--json", "--no-link", *locked,
        "--option", "pure-eval", "true", "--option", "allow-import-from-derivation", "false",
        "--option", "substituters", "https://cache.nixos.org", reference + "#aios-exec"], timeout=600))
    if len(outputs) != 1:
        raise RuntimeError("unexpected executor output count")
    package = outputs[0]["outputs"]["out"]
    executable = str(Path(package) / "bin/aios-execd")
    policy_path = Path(package) / "share/aios/system-approval.json"
    action_path = Path(package) / "share/polkit-1/actions/org.aios.executor.policy"
    for path, original in ((policy_path, release / "crates/aios-exec/policy/system-approval.json"),
        (action_path, release / "crates/aios-exec/policy/org.aios.executor.policy")):
        if path.is_symlink() or path.stat().st_uid != 0 or path.stat().st_mode & 0o222 or path.read_bytes() != original.read_bytes():
            raise RuntimeError("packaged approval policy does not match protected compiled source")
    action_document = ET.fromstring(action_path.read_bytes())
    actions = action_document.findall("action")
    if [action.attrib["id"] for action in actions] != ["org.aios.executor.activate-exact-plan", "org.aios.executor.activate-exact-plan-elevated"]:
        raise RuntimeError("unexpected polkit action catalog")
    for action in actions:
        if action.findall("annotate") or any(action.findtext("defaults/" + key) != "auth_admin" for key in ("allow_any","allow_inactive","allow_active")):
            raise RuntimeError("cached or implicit polkit authorization is forbidden")
    denials = []
    payload = b'{"uid":0,"approved":true,"command":"true","store_path":"/nix/store/arbitrary"}'
    for name, arguments, forged in (("no-unauthenticated-rpc", [], {}),
        ("no-shell-route", ["--execute"], {}),
        ("no-caller-template-path", ["--template", "/tmp/client.nix"], {}),
        ("no-fixture-override", ["--fixture"], {"SUDO_UID":"0", "SUDO_USER":"root", "AIOS_EXEC_FIXTURE":"1"})):
        response = json.loads(run([executable, *arguments], payload, expected=5, timeout=10, extra=forged))
        if response != {"schema_version":1,"error":"BROKER_AUTHORITY_REQUIRED"}:
            raise RuntimeError("unprivileged broker invocation gained authority")
        denials.append({"name":name,"actual_uid":os.getuid(),"exit":5,"response":response})
    closure = json.loads(run(["nix","path-info","--json","--recursive",package]))
    print("AIOS_BROKER_PREPARATION " + json.dumps({"evidence_kind":"real-guest-rust-filesystem-sqlite-fixtures-and-package-denials",
        "target_identity":identity,"package":package,"executable_sha256":hashlib.sha256(Path(executable).read_bytes()).hexdigest(),
        "fixture_tests_passed":54,"actual_read_only_bus_tests_passed":2,"actual_polkit_availability_tests_passed":1,
        "actual_read_only_polkit_owner_tests_passed":int(polkit[0]["owner_identity_verified"]),"native_caller_observations":observations[0],"polkit_observations":polkit[0],
        "approval_policy_sha256":hashlib.sha256(policy_path.read_bytes()).hexdigest(),"polkit_action_sha256":hashlib.sha256(action_path.read_bytes()).hexdigest(),"denials":denials,"runtime_closure":closure,"commands":commands,
        "installed_root_template_verified":False,"root_candidate_registration_verified":False,
        "root_ledger_execution_verified":False,"native_target_runtime_verified":False,"authenticated_bus_peer_verified":False,"isolated_worker_verified":False,"system_activation_verified":False,
        "limitations":["Filesystem/SQLite tests execute the real library as the dev UID with fixture templates, grants and build output.",
        "Production constructors have no fixture/environment override; actual root runtime remains unavailable.",
        "Actual dev-UID bus credentials/process/logind and connection-change denials are read-only checks; no production root VerifiedCaller is minted.",
        "Polkit availability is observed on the running image. Absent service proves refusal only; owner/catalog checks run only when the native service is installed. No CheckAuthorization or trusted UI confirmation occurs.",
        "Native target/config and caller intake are implemented but root runtime remains unqualified; original user-daemon forwarding, builder output/stop proofs, polkit, guard and recovery remain to connect/qualify."]}, sort_keys=True))


if __name__ == "__main__":
    main()
