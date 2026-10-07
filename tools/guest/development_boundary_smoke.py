#!/usr/bin/env python3
"""Pure module evaluations and actual unprivileged installed-package denials.

No root configuration is activated; source-copy fixtures run as the guest dev UID.
"""
import json
import os
from pathlib import Path
import subprocess
import base64
import hashlib
import sys


def main():
    release = Path(__file__).resolve().parents[2]
    subprocess.run(["bash", "-n", str(release / "dev/seed/bootstrap.sh")], check=True, timeout=10)
    sys.path.insert(0, str(release / "tools"))
    from aios_dev.vm import finish_console_command
    import snapshot
    identity = snapshot.identity()
    _, encoded_finish = finish_console_command({"plan":{"guest_uuid":identity["dmi_uuid"],"installation_uuid":identity["installation_uuid"]}})
    finish = base64.b64decode(encoded_finish, validate=True)
    subprocess.run(["bash", "-n"], input=finish, check=True, timeout=10)
    initial_probe = subprocess.run(["python3", str(release / "tools/guest/initial_preflight.py")], capture_output=True, timeout=10)
    if initial_probe.returncode != 5 or json.loads(initial_probe.stdout) != {"schema_version":1,"error":"INITIAL_PREFLIGHT_ROOT_REQUIRED"}:
        raise RuntimeError("initial native preflight allowed nonroot execution")
    reference = "path:" + str(release)
    locked = ["--no-update-lock-file", "--no-write-lock-file"]
    cases = json.loads(subprocess.check_output(["nix", "eval", "--json", *locked, reference + "#lib.developmentBoundary"], timeout=120))
    for name in ("development", "disabled", "production", "sessionHeadless", "sessionDesktop",
                 "productHeadless", "productDesktop"):
        if cases[name]["failedAssertions"]:
            raise RuntimeError("valid module case rejected: " + name + " " + json.dumps(cases[name]["failedAssertions"]))
    required_denials = {
        "wrongRole":"Developer authority is limited to explicitly marked development guests.",
        "missingUuid":"Developer authority requires canonical DMI and installation UUIDs.",
        "extraNixTrust":"Only root may be a Nix trusted-user; developer code deployment is a separate authority.",
        "wheelDeveloper":"The dedicated dev account must be normal and have no wheel authority.",
        "wheelModel":"The product model must have no developer/admin authority.",
        "productionHelper":"Production excludes VM-only developer authority.",
        "productionPasswordlessWheel":"Production excludes unrestricted passwordless wheel sudo.",
        "productionSudoHelper":"Production excludes developer helper rules and unrestricted passwordless sudo.",
        "productionPasswordlessAll":"Production excludes developer helper rules and unrestricted passwordless sudo.",
        "productionDesktopAutologin":"Production excludes graphical and console acceptance autologin.",
        "productionConsoleAutologin":"Production excludes graphical and console acceptance autologin.",
        "productionDevAccount":"Production excludes reserved development and tester accounts.",
        "productionTesterAccount":"Production excludes reserved development and tester accounts.",
        "sessionMissingPackage":"Enabled Horizon OS user broker requires the reviewed aiosCore package.",
        "visualWithoutDesktop":"Horizon OS visual control requires desktop.enable.",
        "initrdWithoutRecovery":"Horizon OS initrd diagnostics require recovery.enable.",
    }
    for name, reason in required_denials.items():
        if reason not in cases[name]["failedAssertions"]:
            raise RuntimeError("invalid module case missing the required denial: " + name)
    for name in ("disabled", "production"):
        if cases[name]["helperPresent"] or cases[name]["developerRules"] or cases[name]["developmentEnabled"]:
            raise RuntimeError("developer authority leaked to nondevelopment case")
        if cases[name]["sessionEnabled"] or cases[name]["sessionWantedBy"] or cases[name]["processWantedBy"]:
            raise RuntimeError("disabled broker was registered for startup")
    for name in ("sessionHeadless", "sessionDesktop"):
        case = cases[name]
        if (not case["sessionEnabled"] or case["sessionWantedBy"] != ["default.target"]
                or case["sessionOverride"] != "asDropin" or set(case["trustedUsers"]) != {"root"}
                or case["processWantedBy"] != ["default.target"] or case["processOverride"] != "asDropin"
                or case["modelEnabled"] or case["modelAccess"] or case["helperPresent"]
                or case["developmentEnabled"] or case["developerRules"]
                or not any(path.endswith("-aios-core-0.1.0") for path in case["unitPackages"])):
            raise RuntimeError("user broker acquired unexpected authority or lost unit wiring: " + name + " " + json.dumps(case, sort_keys=True))
        if any("aios-model.service" in group or "aios-model.socket" in group for group in case["bootRequires"].values()):
            raise RuntimeError("broker module made boot/login/connectivity require inference")
    if cases["sessionHeadless"]["desktopEnabled"] or not cases["sessionDesktop"]["desktopEnabled"]:
        raise RuntimeError("user broker requires an unintended desktop configuration")
    disabled = cases["disabled"]
    if disabled["aiosEnabled"] or disabled["productOptions"] != {
            "users":[], "desktop":False, "visualControl":False, "index":True,
            "proactive":False, "automation":True, "recovery":False,
            "initrdDiagnostics":False, "kernelProbes":False,
            "guardTimeoutSeconds":180, "keepKnownGoodGenerations":3}:
        raise RuntimeError("reusable product option defaults changed: " + json.dumps(disabled, sort_keys=True))
    headless = cases["productHeadless"]
    desktop = cases["productDesktop"]
    if (not headless["aiosEnabled"] or not headless["sessionEnabled"] or headless["modelEnabled"]
            or headless["productOptions"]["users"] != ["alice"] or headless["productOptions"]["desktop"]
            or not desktop["productOptions"]["desktop"]):
        raise RuntimeError("product headless/desktop composition changed")
    dev = cases["development"]
    expected = [{"users":["dev"],"groups":[],"host":"ALL","runAs":"root:root","commands":[{"command":"/run/current-system/sw/bin/aios-dev-deploy --request-stdin","options":["NOPASSWD","NOSETENV"]}]}]
    if not dev["helperPresent"] or dev["trustedUsers"] != ["root"] or dev["developerRules"] != expected:
        raise RuntimeError("development authority is not the exact dedicated rule")
    outputs = json.loads(subprocess.check_output(["nix", "build", "--json", "--no-link", *locked, reference + "#aios-dev-deploy"], timeout=300))
    if len(outputs) != 1:
        raise RuntimeError("unexpected helper output count")
    package = outputs[0]["outputs"]["out"]
    executable = str(Path(package) / "bin/aios-dev-deploy")
    attempts = []
    for env in ({}, {"SUDO_USER":"dev","SUDO_UID":str(os.getuid()),"SUDO_GID":str(os.getgid()),"PYTHONPATH":"/tmp","PYTHONHOME":"/tmp"}):
        response = subprocess.run([executable, "--request-stdin"], input=b'{"command":"true"}', capture_output=True, env={**os.environ, **env}, timeout=10)
        if response.returncode != 5 or json.loads(response.stdout) != {"schema_version":1,"error":"DEVELOPER_AUTHORITY_REQUIRED"}:
            raise RuntimeError("nonroot invocation acquired developer authority")
        attempts.append({"uid":os.getuid(),"forged_sudo_and_python_environment":bool(env),"exit":response.returncode,"response":json.loads(response.stdout)})
    subprocess.run(["python3", "-m", "unittest", "discover", "-s", "tests/unit", "-p", "test_dev_deploy.py", "-v"], cwd=release, check=True, timeout=60)
    print("AIOS_DEVELOPMENT_BOUNDARY " + json.dumps({"evidence_kind":"real-guest-package-and-module-evaluation","module_cases":cases,
        "package":package,"nonroot_denials":attempts,"source_copy_fixture_tests":14,
        "initial_image_script_syntax":{"argv":["bash","-n",str(release / "dev/seed/bootstrap.sh")],"upstream_exit":0,"fresh_installation_verified":False},
        "initial_native_preflight_nonroot_denial":{"argv":["python3",str(release / "tools/guest/initial_preflight.py")],"upstream_exit":5,"actual_uid":os.getuid()},
        "initial_finish_syntax":{"argv":["bash","-n"],"upstream_exit":0,"script_sha256":hashlib.sha256(finish).hexdigest(),"executed":False},
        "root_registration_verified":False,"guarded_activation_verified":False,"production_boot_verified":False,
        "limitations":["Filesystem-copy and target checks are fixtures under the dev UID.","The helper has not been installed in the running system; no VM sudo or activation performed.","Guarded test/commit and production isolation remain unimplemented/unverified."]}, sort_keys=True))


if __name__ == "__main__":
    main()
