#!/usr/bin/env python3
"""Pure module evaluations and actual unprivileged installed-package denials.

No root configuration is activated; source-copy fixtures run as the guest dev UID.
"""
import json
import os
from pathlib import Path
import subprocess


def main():
    release = Path(__file__).resolve().parents[2]
    reference = "path:" + str(release)
    locked = ["--no-update-lock-file", "--no-write-lock-file"]
    cases = json.loads(subprocess.check_output(["nix", "eval", "--json", *locked, reference + "#lib.developmentBoundary"], timeout=120))
    for name in ("development", "disabled", "production"):
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
    }
    for name, reason in required_denials.items():
        if reason not in cases[name]["failedAssertions"]:
            raise RuntimeError("invalid module case missing the required denial: " + name)
    for name in ("disabled", "production"):
        if cases[name]["helperPresent"] or cases[name]["developerRules"] or cases[name]["developmentEnabled"]:
            raise RuntimeError("developer authority leaked to nondevelopment case")
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
        "root_registration_verified":False,"guarded_activation_verified":False,"production_boot_verified":False,
        "limitations":["Filesystem-copy and target checks are fixtures under the dev UID.","The helper has not been installed in the running system; no VM sudo or activation performed.","Guarded test/commit and production isolation remain unimplemented/unverified."]}, sort_keys=True))


if __name__ == "__main__":
    main()
