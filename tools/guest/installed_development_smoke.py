#!/usr/bin/env python3
"""Fixed denial cases against the actual installed development sudo helper."""
import json
import os
from pathlib import Path
import subprocess
import uuid

import snapshot

HELPER = "/run/current-system/sw/bin/aios-dev-deploy"
SUDO = "/run/wrappers/bin/sudo"


def attempt(argv, request, expected_exit, expected_error, *, environment=None):
    payload = request if isinstance(request, bytes) else snapshot.canonical(request)
    response = subprocess.run(argv, input=payload, capture_output=True, timeout=20, env=environment)
    if len(response.stdout) + len(response.stderr) > 65536:
        raise RuntimeError("helper denial output exceeded limit")
    result = snapshot.decode(response.stdout)
    if response.returncode != expected_exit or result != {"schema_version": 1, "error": expected_error}:
        raise RuntimeError("installed helper did not produce required denial")
    return {"argv": argv, "upstream_exit": response.returncode, "response": result}


def main():
    if os.getuid() == 0 or os.geteuid() == 0 or not Path(HELPER).is_file():
        raise RuntimeError("qualification requires the nonroot dev account and installed helper")
    identity = snapshot.identity()
    if any(identity[key] != value for key, value in (("os_id", "nixos"), ("guest_role", "development"),
                                                   ("disk_serial", "AIOS_DEV_ROOT"), ("management_channel", "ssh-development"))):
        raise RuntimeError("qualification target is not the enrolled development VM")
    request = {"schema_version": 1, "operation": "status", "identity": identity, "transaction_id": str(uuid.uuid4())}
    attempts = []
    for forged in (False, True):
        environment = {**os.environ}
        if forged:
            environment.update(SUDO_USER="dev", SUDO_UID=str(os.getuid()), SUDO_GID=str(os.getgid()),
                               PYTHONPATH="/tmp", PYTHONHOME="/tmp")
        result = attempt([HELPER, "--request-stdin"], request, 5, "DEVELOPER_AUTHORITY_REQUIRED", environment=environment)
        attempts.append({"case": "nonroot-forged-environment" if forged else "nonroot", "actual_uid": os.getuid(), **result})
    root_command = [SUDO, "-n", HELPER, "--request-stdin"]
    for field, value in (("os_id", "cachyos"), ("guest_role", "production"), ("disk_serial", "WRONG"),
                         ("management_channel", "other"), ("dmi_uuid", str(uuid.uuid4())),
                         ("installation_uuid", str(uuid.uuid4())), ("boot_id", str(uuid.uuid4())),
                         ("machine_id", "0" * 32), ("current_system", "/nix/store/" + "0" * 32 + "-nixos-system-wrong")):
        wrong = {**request, "identity": {**identity, field: value}}
        attempts.append({"case": "wrong-" + field, **attempt(root_command, wrong, 4, "DEVELOPMENT_TARGET_MISMATCH")})
    for case, malformed in (("shell-field", {**request, "command": "true"}),
                            ("duplicate-schema", b'{"schema_version":1,"schema_version":1}'),
                            ("boolean-schema", {**request, "schema_version": True}),
                            ("product-authority", {**request, "operation": "register", "snapshot_digest": "0" * 64, "authority": "product"})):
        attempts.append({"case": case, **attempt(root_command, malformed, 2, "INVALID_DEVELOPER_REQUEST")})
    attempts.append({"case": "unknown-registration", **attempt(root_command, request, 3, "REGISTRATION_NOT_FOUND")})
    for mode in ("test", "commit"):
        mutation = {**request, "operation": mode, "snapshot_digest": "0" * 64, "authority": "guest-root-code-deployment"}
        attempts.append({"case": mode + "-unavailable", **attempt(root_command, mutation, 9, "GUARDED_ACTIVATION_UNAVAILABLE")})
    if snapshot.identity() != identity:
        raise RuntimeError("development identity changed during denial qualification")
    print("AIOS_INSTALLED_DEVELOPMENT " + json.dumps({"schema_version": 1,
        "evidence_kind": "actual-installed-development-helper-denials", "target_identity": identity,
        "attempts": attempts, "root_registration_verified": False, "activation_performed": False,
        "guarded_activation_verified": False, "product_model_caller_verified": False,
        "limitations": ["Nonroot attempts use the real dev UID, not the product model account.",
                        "Wrong-target and parser denials exercise the installed root helper through its exact sudo rule.",
                        "Successful root registration and guarded activation require separate evidence."]}, sort_keys=True))


if __name__ == "__main__":
    main()
