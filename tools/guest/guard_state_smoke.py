#!/usr/bin/env python3
"""Build the guard and check its pure plan interface in a verified guest.

Plan identities, paths and health data are fixtures. This never activates a
system or arms a privileged guard. Formatting output is reviewable source data.
"""
import copy
import hashlib
import json
from pathlib import Path
import subprocess


def fixture_plan():
    def closure(name):
        return {"path": "/nix/store/" + "a" * 32 + "-" + name,
                "activation_sha256": "1" * 64, "kernel_sha256": "2" * 64,
                "initrd_sha256": "3" * 64, "boot_adapter_sha256": "4" * 64}
    return {"schema_version": 1,
            "transaction_id": "11111111-1111-4111-8111-111111111111",
            "identity": {"installation_uuid": "22222222-2222-4222-8222-222222222222",
                         "dmi_uuid": "33333333-3333-4333-8333-333333333333",
                         "boot_id": "44444444-4444-4444-8444-444444444444",
                         "machine_id": "c" * 32, "role": "development",
                         "disk_serial": "AIOS_FIXTURE", "management_channel": "ssh-development"},
            "source_digest": "6" * 64, "candidate_digest": "7" * 64,
            "nixpkgs_revision": "774debe7a0d1b496e35677ad955a1011c6ff74f3",
            "prior": {"running": closure("running"), "profile": closure("profile"),
                      "boot": closure("boot"), "managed_sha256": "8" * 64, "model": None},
            "candidate": {"kind": "system", "closure": closure("candidate")},
            "managed_sha256": "9" * 64, "retained_guard_sha256": "a" * 64,
            "guard_timeout_seconds": 180,
            "health": {"required_mounts": ["/", "/nix"],
                       "baseline_units": [{"name": name, "active": True, "failed": False}
                                          for name in ("sshd.service", "dbus.service")],
                       "required_apis": [{"api": api, "uid": None, "healthy": True}
                                         for api in ("executor", "graph")],
                       "required_user_units": []}}


def main():
    release = Path(__file__).resolve().parents[2]
    reference = "path:" + str(release)
    locked = ["--no-update-lock-file", "--no-write-lock-file"]
    formatted = []
    for relative in ("crates/aios-guard/src/lib.rs", "crates/aios-guard/src/main.rs",
                     "crates/aios-guard/src/activation.rs", "crates/aios-guard/src/native.rs",
                     "crates/aios-exec/src/native.rs", "crates/aios-exec/src/health.rs",
                     "crates/aios-exec/src/health/tests.rs", "crates/aios-guard/tests/guard.rs"):
        original = (release / relative).read_bytes()
        result = subprocess.check_output(["nix", "develop", *locked, reference,
            "--command", "rustfmt", "--edition", "2024", "--emit", "stdout",
            "--config", "skip_children=true"], input=original, timeout=120)
        formatted.append({"path": relative, "source_sha256": hashlib.sha256(original).hexdigest(),
                          "formatted_sha256": hashlib.sha256(result).hexdigest(),
                          "formatted_source": result.decode("utf-8")})
    print("AIOS_GUARD_FORMAT " + json.dumps(formatted, sort_keys=True), flush=True)
    subprocess.run(["nix", "develop", *locked, reference, "--command", "cargo", "test", "--locked",
        "-p", "aios-exec", "health::tests::actual_pid1_bus_and_fixed_units_are_readonly", "--", "--nocapture"],
        check=True, timeout=180)
    outputs = json.loads(subprocess.check_output(["nix", "build", "--json", "--no-link",
        *locked, reference + "#aios-guard"], timeout=600))
    if len(outputs) != 1:
        raise RuntimeError("unexpected guard output count")
    package = outputs[0]["outputs"]["out"]
    executable = str(Path(package) / "bin/aios-guard")
    checks = []

    def check(name, payload, expected, args=("--check-plan",)):
        result = subprocess.run([executable, *args], input=payload, capture_output=True, timeout=10)
        if result.returncode != expected:
            raise RuntimeError("guard check returned unexpected exit: " + name)
        response = json.loads(result.stdout)
        if expected == 2 and response != {"schema_version": 1, "error": "INVALID_GUARD_PLAN"}:
            raise RuntimeError("invalid plan denial changed: " + name)
        if expected == 9 and response != {"schema_version": 1, "error": "GUARD_RUNTIME_ADAPTER_UNAVAILABLE"}:
            raise RuntimeError("unqualified runtime acquired an execution path")
        if expected == 5 and response != {"schema_version": 1, "error": "GUARD_NATIVE_INTAKE_FAILED", "reason": "Authority"}:
            raise RuntimeError("nonroot native guard intake was not denied")
        checks.append({"name": name, "exit": result.returncode, "response": response})
        return response

    plan = fixture_plan()
    valid = check("valid-fixture", json.dumps(plan).encode(), 0)
    reordered = check("key-order-independent", json.dumps(plan, sort_keys=True).encode(), 0)
    if valid != reordered or valid["activation_performed"] or valid["runtime_adapter_available"]:
        raise RuntimeError("pure checker claimed activation or changed canonical digest")
    for name, field, value in (("unknown-command", "command", "true"),
                               ("wrong-version", "schema_version", 2),
                               ("wrong-upstream", "nixpkgs_revision", "0" * 40)):
        invalid = copy.deepcopy(plan)
        invalid[field] = value
        check(name, json.dumps(invalid).encode(), 2)
    check("duplicate-key", b'{"schema_version":1,' + json.dumps(plan).encode()[1:], 2)
    check("oversize", b" " * 65537, 2)
    check("no-live-runtime", b"", 9, args=())
    check("no-command-fallback", b'{"command":"true"}', 9, args=("--execute",))
    check("actual-nonroot-native-intake", b"", 5, args=("--native-preflight",))
    reboot = copy.deepcopy(plan)
    reboot["candidate"]["closure"]["kernel_sha256"] = "f" * 64
    if not check("separate-reboot", json.dumps(reboot).encode(), 0)["reboot_required"]:
        raise RuntimeError("kernel change bypassed reboot classification")
    closure_info = json.loads(subprocess.check_output(["nix", "path-info", "--json", "--recursive", package], timeout=30))
    current_wrapper = Path("/run/current-system/bin/switch-to-configuration")
    wrapper = current_wrapper.read_text()
    print("AIOS_GUARD_STATE " + json.dumps({"evidence_kind": "real-guest-package-with-fixture-plans",
        "installed_activation_wrapper": {"path":str(current_wrapper.resolve(strict=True)),
            "sha256":hashlib.sha256(current_wrapper.read_bytes()).hexdigest(),
            "boot_adapter_exports":[line for line in wrapper.splitlines() if line.startswith("export INSTALL_BOOTLOADER=")]},
        "package": package, "executable_sha256": hashlib.sha256(Path(executable).read_bytes()).hexdigest(),
        "checks": checks, "runtime_closure": closure_info, "real_activation_verified": False,
        "independent_guard_survival_verified": False, "authenticated_heartbeat_verified": False,
        "limitations": ["Plan/health/identity inputs are fixtures.",
                        "No privileged adapter, guard unit, system activation or boot write runs in this test."]}, sort_keys=True))


if __name__ == "__main__":
    main()
