#!/usr/bin/env python3
"""Qualify the bounded native read of the actual installed KDE polkit file."""
import json
import os
from pathlib import Path
import subprocess
import snapshot


def main():
    identity = snapshot.identity()
    release = Path(__file__).resolve().parents[2]
    argv = ["nix", "develop", "--no-update-lock-file", "--no-write-lock-file",
        "path:" + str(release), "--command", "cargo", "test", "--locked", "-p", "aios-exec", "--lib",
        "approval::policy::tests::installed_udisks_policy_bounds", "--", "--ignored", "--exact", "--nocapture"]
    environment = {**os.environ, "NIX_USER_CONF_FILES":"/dev/null", "NIX_REMOTE":"daemon"}
    result = subprocess.run(argv, capture_output=True, env=environment, timeout=300, check=False)
    print(result.stdout.decode(errors="replace"), end="", flush=True)
    print(result.stderr.decode(errors="replace"), end="", flush=True)
    if result.returncode or snapshot.identity() != identity:
        raise RuntimeError("installed policy bound qualification failed or target changed")
    observations = [snapshot.decode(line.split("AIOS_INSTALLED_POLICY_BOUND ",1)[1])
        for line in result.stdout.decode().splitlines() if "AIOS_INSTALLED_POLICY_BOUND " in line]
    if len(observations) != 1 or b"1 passed; 0 failed" not in result.stdout:
        raise RuntimeError("installed policy evidence missing")
    print("AIOS_INSTALLED_POLICY " + json.dumps({"target_identity":identity,"argv":argv,
        "upstream_exit":result.returncode,"observation":observations[0],
        "native_root_preflight_verified":False,"native_authorization_verified":False},sort_keys=True),flush=True)


if __name__ == "__main__":
    main()
