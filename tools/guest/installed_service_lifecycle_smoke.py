#!/usr/bin/env python3
"""Request and verify the fixed disposable-image service lifecycle qualification."""
import json
import os
from pathlib import Path
import secrets
import stat
import time
import snapshot

MARKER = Path("/tmp/aios-service-lifecycle-request")
REPORT = Path("/run/aios-service-lifecycle/report.json")


def main():
    identity = snapshot.identity()
    if Path("/etc/aios/desktop-test-profile").read_text() != "synthetic-disposable-plasma-wayland-v1\n":
        raise RuntimeError("service lifecycle probe requires the disposable desktop image")
    token = secrets.token_hex(16)
    try:
        descriptor = os.open(MARKER, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(descriptor, "w") as marker:
            marker.write(token + "\n")
        marker_state = MARKER.lstat()
        if marker_state.st_uid != os.geteuid() or stat.S_IMODE(marker_state.st_mode) != 0o600:
            raise RuntimeError("lifecycle request marker identity changed")
        deadline = time.monotonic() + 150
        report = None
        while time.monotonic() < deadline:
            if REPORT.exists():
                candidate = json.loads(REPORT.read_text())
                if candidate.get("request_token") == token:
                    report = candidate
                    break
            time.sleep(0.2)
        if report is None:
            raise RuntimeError("fixed service lifecycle fixture did not publish a fresh result")
        if report.get("verified") is not True:
            raise RuntimeError("fixed service lifecycle fixture failed: "
                               + json.dumps(report.get("error"), sort_keys=True))
        current_identity = snapshot.identity()
        report_identity = report.get("identity")
        if current_identity != identity or report_identity != identity:
            raise RuntimeError("lifecycle result is not bound to the current enrolled target: "
                               + json.dumps({"initial": identity, "current": current_identity,
                                             "report": report_identity}, sort_keys=True))
        if report.get("model_active") is not False or report.get("tester_runtime_removed") is not True:
            raise RuntimeError("model-independent or logout lifecycle invariant failed")
        if [item.get("unit") for item in report.get("system_restarts", [])] != [
            "aios-state.service", "aios-observer.service", "aios-build.service",
            "aios-execd.service",
        ]:
            raise RuntimeError("system AI service restart coverage is incomplete")
        if [item.get("unit") for item in report.get("user_restarts", [])] != ["aios-sessiond.service", "aios-processd.service", "aios-ui-agent.service"]:
            raise RuntimeError("user AI service restart coverage is incomplete")
        expected = {"sshd.service", "NetworkManager.service", "display-manager.service"}
        if set(report.get("infrastructure_before", {})) != expected or set(report.get("infrastructure_after", {})) != expected:
            raise RuntimeError("login, management or network availability evidence is incomplete")
        access = report.get("access_plan", {})
        if set(access.get("accounts", {})) != {"aios-state", "aios-observer", "aios-builder"}:
            raise RuntimeError("service-account access evidence is incomplete")
        if len(access.get("unit_policies", [])) != 7 or len(access.get("private_paths", [])) != 16:
            raise RuntimeError("unit or private-path access evidence is incomplete")
        expected_ai_units = {
            "aios-state.service", "aios-observer.service", "aios-build.service",
            "aios-execd.service", "aios-sessiond.service", "aios-processd.service",
            "aios-ui-agent.service",
        }
        if set(access.get("settled_services", {})) != expected_ai_units:
            raise RuntimeError("stable service evidence is incomplete")
        observer = access.get("observer_status", {})
        if (observer.get("systemd_subscription") is not True
                or observer.get("device_subscription") is not True
                or observer.get("mutation_authority") is not False
                or observer.get("network_egress") is not False):
            raise RuntimeError("native observer evidence is incomplete")
        builder = access.get("candidate_builder", {}).get("data", {})
        if (builder.get("candidate_build_authority") is not True
                or builder.get("activation_authority") is not False
                or builder.get("candidate_template_selection") is not False
                or builder.get("nix_trusted_user") is not False
                or builder.get("network_egress") is not False
                or access.get("nix_trusted_users") != ["root"]):
            raise RuntimeError("candidate builder authority evidence is incomplete")
        if set(access.get("documented_exceptions", {})) != {
            "aios-state.service", "aios-observer.service", "aios-build.service",
            "aios-execd.service", "aios-processd.service", "aios-ui-agent.service",
        }:
            raise RuntimeError("documented service exceptions are incomplete")
        print("AIOS_INSTALLED_SERVICE_LIFECYCLE " + json.dumps(report, sort_keys=True), flush=True)
    finally:
        try:
            state = MARKER.lstat()
            if stat.S_ISREG(state.st_mode) and state.st_uid == os.geteuid():
                MARKER.unlink()
        except FileNotFoundError:
            pass


if __name__ == "__main__":
    main()
