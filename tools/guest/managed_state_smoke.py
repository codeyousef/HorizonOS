#!/usr/bin/env python3
"""Real locked catalog/package/module checks; manifests and machine are fixtures.

No root candidate registration, system activation, service start or approval runs.
"""
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import snapshot as source


def main():
    release = Path(__file__).resolve().parents[2]
    reference = "path:" + str(release)
    locked = ["--no-update-lock-file", "--no-write-lock-file"]
    pure = ["--option", "pure-eval", "true", "--option", "allow-import-from-derivation", "false"]
    identity = source.identity()
    environment = {key: os.environ[key] for key in ("HOME", "PATH", "TMPDIR", "XDG_CACHE_HOME") if key in os.environ}
    environment.update(LANG="C.UTF-8", NIX_USER_CONF_FILES="/dev/null", NIX_REMOTE="daemon")
    commands = []

    def run(argv, payload=None, expected=0, timeout=180):
        if source.identity() != identity:
            raise RuntimeError("guest identity changed before managed-state check")
        result = subprocess.run(argv, input=payload, capture_output=True, env=environment, timeout=timeout)
        commands.append({"argv": argv, "upstream_exit": result.returncode,
                         "stdout": result.stdout.decode(errors="replace"), "stderr": result.stderr.decode(errors="replace")})
        if result.returncode != expected:
            print("AIOS_MANAGED_STATE_FAILURE " + json.dumps(commands, sort_keys=True), flush=True)
            raise RuntimeError("managed-state subprocess exit differs from expected")
        return result.stdout

    formatted = []
    for relative in ("crates/aios-state/src/lib.rs", "crates/aios-state/src/main.rs", "crates/aios-state/tests/state.rs"):
        original = (release / relative).read_bytes()
        output = run(["nix", "develop", *locked, reference, "--command", "rustfmt", "--edition", "2024", "--emit", "stdout", "--config", "skip_children=true"], original)
        formatted.append({"path": relative, "source_sha256": hashlib.sha256(original).hexdigest(),
                          "formatted_sha256": hashlib.sha256(output).hexdigest(), "formatted_source": output.decode()})
    print("AIOS_MANAGED_STATE_FORMAT " + json.dumps(formatted, sort_keys=True), flush=True)
    contract = json.loads(run(["nix", "eval", "--json", *locked, *pure, reference + "#lib.stateContract"]))
    cases = json.loads(run(["nix", "eval", "--json", *locked, *pure, reference + "#lib.managedState"]))
    for name in ("defaults", "packages", "postgresql"):
        if cases[name]["failedAssertions"]:
            raise RuntimeError("valid managed module case rejected: " + name + ": " + str(cases[name]["failedAssertions"]))
        if cases[name]["stateVersion"] != "26.05" or cases[name]["openssh"] != {"enabled":True,"open_firewall":True}:
            raise RuntimeError("protected installation baseline changed")
    if sorted(cases["packages"]["packages"]) != ["kate", "kcalc"]:
        raise RuntimeError("catalog IDs did not resolve to actual system package declarations")
    pg = cases["postgresql"]["postgresql"]
    if not pg["enabled"] or not pg["version"].startswith("17.") or pg["listen"] != "" or pg["tcp"] or pg["authentication"] != "local all all peer":
        raise RuntimeError("PostgreSQL template is not the fixed Unix-only approved major")
    for name, message in (
        ("changedStateVersion","AIOS managed updates preserve the installation stateVersion baseline."),
        ("closedFirewall","AIOS cannot disable the protected management transport."),
        ("broadUnfree","AIOS does not permit a broad allowUnfree override."),
    ):
        if message not in cases[name]["failedAssertions"]:
            raise RuntimeError("trusted module override bypassed protection: " + name)
    if any(cases["denials"].values()):
        raise RuntimeError("invalid managed template input was accepted")
    outputs = json.loads(run(["nix", "build", "--json", "--no-link", *locked, *pure,
        "--option", "substituters", "https://cache.nixos.org", reference + "#aios-state"], timeout=600))
    if len(outputs) != 1:
        raise RuntimeError("unexpected checker output count")
    package = outputs[0]["outputs"]["out"]
    executable = str(Path(package) / "bin/aios-state-check")
    actual_catalog = json.loads(run([executable, "--catalog"]))
    defaults = json.loads(run([executable, "--defaults"]))
    if actual_catalog != contract["catalog"] or defaults != contract["defaults"]:
        raise RuntimeError("installed Rust catalog and template catalog/defaults diverge")
    checks = []

    def check(name, state, expected=0, payload=None, args=("--check-manifest",)):
        result = json.loads(run([executable, *args], json.dumps(state).encode() if payload is None else payload, expected=expected, timeout=10))
        if expected == 0:
            canonical = json.dumps(result["managed"], sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
            if hashlib.sha256(canonical).hexdigest() != result["managed_sha256"] or result["activation_performed"] or result["authorization_granted"]:
                raise RuntimeError("checker falsely claimed authority or inconsistent digest")
        checks.append({"name": name,"exit":expected,"response":result})
        return result

    normalized = check("materialized-defaults", defaults)
    if json.dumps(normalized["managed"], sort_keys=True, separators=(",", ":")) != cases["defaults"]["managedJSON"]:
        raise RuntimeError("canonical Rust bytes differ from Nix imported bytes")
    minimal = {key:defaults[key] for key in ("schema_version","base_template_revision","catalog_revision")}
    if check("omitted-defaults", minimal) != normalized:
        raise RuntimeError("documented defaults changed by key omission")
    installed = copy.deepcopy(defaults); installed["system_packages"] = ["kcalc","kate"]
    normalized = check("approved-applications", installed)
    if json.dumps(normalized["managed"], sort_keys=True, separators=(",", ":")) != cases["packages"]["managedJSON"]:
        raise RuntimeError("package normalization diverges from template")
    for field, value in (("imports",["/var/lib/aios/mutable.nix"]), ("system_packages",["pkgs.runCommand"]),
                         ("catalog_revision","0"*64), ("base_template_revision","0"*64), ("approved",True)):
        invalid = copy.deepcopy(defaults); invalid[field] = value; check("denied-" + field,invalid,2)
    for field in ("enabled","open_firewall"):
        invalid = copy.deepcopy(defaults); invalid["services"]["openssh"][field] = False; check("protected-ssh-" + field,invalid,2)
    check("duplicate-key",defaults,2,payload=b'{"schema_version":1,' + json.dumps(defaults).encode()[1:])
    check("oversized",defaults,2,payload=b" "*65537)
    check("no-execution-path",defaults,2,args=("--execute",))
    previews = []
    request = {"managed": defaults, "intent": {"action":"install_package","package_id":"kate"}}
    response = json.loads(run([executable,"--preview"],json.dumps(request).encode()))
    preview = response["preview"]
    if response["source"] != "offline_manifest" or response["installed_baseline_verified"] or response["activation_performed"] or response["authorization_granted"] or preview["final_authorization_ready"] or preview["added_packages"] != ["kate"]:
        raise RuntimeError("preliminary package preview claimed final authority")
    if any(preview[key] is not None for key in ("candidate_closure","download_bytes","build_bytes","reboot_required","retained_dependency_paths")):
        raise RuntimeError("preliminary preview fabricated build effects")
    previews.append({"name":"install-kate","response":response})
    remove = {"managed":preview["candidate_manifest"],"intent":{"action":"remove_package","package_id":"kate"}}
    response = json.loads(run([executable,"--preview"],json.dumps(remove).encode()))
    if response["preview"]["removed_packages"] != ["kate"] or response["preview"]["user_data_deleted"]:
        raise RuntimeError("removal preview confused declaration removal with data deletion")
    previews.append({"name":"remove-kate-preserves-data","response":response})
    for name, invalid in (("no-client-grant",{**request,"approved":True}),
                          ("unknown-database-state",{"managed":defaults,"intent":{"action":"set_postgresql","enabled":True,"package_id":"postgresql-17"}})):
        response = json.loads(run([executable,"--preview"],json.dumps(invalid).encode(),expected=2))
        if response.get("error") != "INVALID_PREVIEW_REQUEST":
            raise RuntimeError("offline preview accepted caller authorization/data evidence")
        previews.append({"name":name,"exit":2,"response":response})
    closure = json.loads(run(["nix","path-info","--json","--recursive",package]))
    if source.identity() != identity:
        raise RuntimeError("guest identity changed after managed-state checks")
    print("AIOS_MANAGED_STATE " + json.dumps({"evidence_kind":"real-guest-locked-catalog-module-and-package-with-fixture-manifests",
        "target_identity":identity,"catalog":contract["catalog"],"module_cases":cases,"checks":checks,"previews":previews,
        "package":package,"executable_sha256":hashlib.sha256(Path(executable).read_bytes()).hexdigest(),
        "runtime_closure":closure,"commands":commands,"root_candidate_registration_verified":False,
        "system_activation_verified":False,"application_capabilities_verified":False,"postgresql_readiness_verified":False,
        "power_policy_applied_verified":False,"limitations":["NixOS machine/manifest and Rust permission/data tests are fixtures.",
        "Catalog metadata and checker build/run are actual pinned Nix evaluation and execution.",
        "Root ownership, candidate registration, runtime power application, database readiness and desktop capabilities remain to qualify."]}, sort_keys=True))


if __name__ == "__main__":
    main()
