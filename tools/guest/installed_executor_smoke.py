#!/usr/bin/env python3
"""Fixed Executor1 probe in the caller's live authenticated SSH login session."""
import json
import os
from pathlib import Path
import subprocess
import snapshot
import installed_runtime_smoke


def main():
    identity = snapshot.identity()
    installed_runtime_smoke.main()
    release = Path(__file__).resolve().parents[2]
    argv = ["nix", "develop", "--no-update-lock-file", "--no-write-lock-file", "path:" + str(release),
        "--command", "cargo", "test", "--locked", "-p", "aios-exec", "--test", "installed_transport",
        "--", "--ignored", "--nocapture"]
    result = subprocess.run(argv, capture_output=True, timeout=600, check=False)
    if snapshot.identity() != identity:
        raise ValueError("runtime target changed during installed transport check")
    if len(result.stdout) + len(result.stderr) > 1024 * 1024:
        raise ValueError("installed transport output exceeds bound")
    print(result.stdout.decode(errors="replace"), flush=True)
    print(result.stderr.decode(errors="replace"), flush=True)
    print("AIOS_INSTALLED_EXECUTOR_COMMAND " + json.dumps({"argv":argv,"upstream_exit":result.returncode,
        "target_identity":identity,"uid":os.getuid(),"caller_session_held_open":True},sort_keys=True),flush=True)
    if result.returncode or b"1 passed; 0 failed; 0 ignored" not in result.stdout:
        raise RuntimeError("installed transport qualification failed")


if __name__ == "__main__":
    main()
