#!/usr/bin/env python3
"""Exercise installed deterministic CLI paths in one live headless SSH session."""
import json
import os
from pathlib import Path
import subprocess

import snapshot


CLI = Path("/run/current-system/sw/bin/aiosctl")
LIMIT = 1024 * 1024


def call(*arguments, expected=0):
    result = subprocess.run(
        [str(CLI), *arguments],
        stdin=subprocess.DEVNULL,
        capture_output=True,
        timeout=45,
        check=False,
        env={
            "HOME": os.environ["HOME"],
            "LANG": "C.UTF-8",
            "PATH": "/run/current-system/sw/bin",
            "XDG_RUNTIME_DIR": os.environ["XDG_RUNTIME_DIR"],
            "DBUS_SESSION_BUS_ADDRESS": os.environ["DBUS_SESSION_BUS_ADDRESS"],
        },
    )
    if len(result.stdout) + len(result.stderr) > LIMIT:
        raise ValueError("CLI output exceeds bound")
    if result.returncode != expected:
        raise RuntimeError(
            f"aiosctl {arguments[0]} exited {result.returncode}, expected {expected}: "
            + result.stderr.decode(errors="replace")
        )
    return json.loads(result.stdout)


def main():
    identity = snapshot.identity()
    if os.getuid() == 0 or any(os.environ.get(name) for name in ("DISPLAY", "WAYLAND_DISPLAY")):
        raise ValueError("installed CLI qualification requires a non-root headless session")
    executable = CLI.resolve(strict=True)
    if not str(executable).startswith("/nix/store/") or executable.name != "aiosctl":
        raise ValueError("aiosctl is not the installed immutable executable")

    automation = call("automation", "list", "--json")
    if automation != {
        "schema_version": 1,
        "operation": "automation_list",
        "data": {
            "definitions": [],
            "owner": "authenticated_client",
            "persistent": False,
            "scheduling_available": False,
        },
        "mutation_performed": False,
    }:
        raise ValueError("automation list contract changed")

    plan = call("plan", "Install Blender", "--json")
    data = plan.get("data", {})
    plan_id = data.get("plan_id")
    plan_hash = data.get("plan_sha256")
    if (
        plan.get("operation") != "prepared_plan"
        or not isinstance(plan_id, str)
        or not isinstance(plan_hash, str)
        or len(plan_hash) != 64
        or data.get("status", {}).get("state") != "PLANNED"
        or data.get("system_effects_performed") is not False
    ):
        raise ValueError("prepared plan contract changed")

    transaction = call("transaction", "inspect", plan_id, "--json")
    if (
        transaction.get("operation") != "transaction"
        or transaction.get("data", {}).get("status", {}).get("plan_id") != plan_id
        or transaction.get("data", {}).get("status", {}).get("state") != "PLANNED"
        or transaction.get("data", {}).get("system_effects_performed") is not False
    ):
        raise ValueError("transaction inspection contract changed")

    denials = {}
    for operation in ("authorize", "apply"):
        denial = call("transaction", operation, plan_id, "--json", expected=1)
        if (
            denial.get("error", {}).get("code") != "AUTH_REQUIRED"
            or denial.get("data", {}).get("plan_id") != plan_id
        ):
            raise ValueError(f"headless {operation} did not preserve the concrete plan")
        denials[operation] = denial["error"]["code"]

    if snapshot.identity() != identity:
        raise ValueError("runtime target changed during installed CLI check")
    print(
        "AIOS_INSTALLED_CLI="
        + json.dumps(
            {
                "evidence_kind": "real-installed-headless-cli",
                "uid": os.getuid(),
                "boot_id": identity["boot_id"],
                "installed_executable": str(executable),
                "headless": True,
                "plan_id": plan_id,
                "plan_sha256": plan_hash,
                "transaction_state": "PLANNED",
                "denials": denials,
                "automation_list_verified": True,
                "system_effects_performed": False,
            },
            sort_keys=True,
        ),
        flush=True,
    )


if __name__ == "__main__":
    main()
