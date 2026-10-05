#!/usr/bin/env python3
"""Fixed installed process probe retaining its authenticated SSH login."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import snapshot


def main():
    identity = snapshot.identity()
    release = Path(__file__).resolve().parents[2]
    argv = ["nix", "develop", "--no-update-lock-file", "--no-write-lock-file", "path:" + str(release),
        "--command", "cargo", "test", "--locked", "-p", "aios-session", "--test", "process_observer",
        "--", "--ignored", "--nocapture"]
    # The host publishes immutable source; generated files stay in the VM.
    # Keep this foreground process alive until the real caller has finished.
    with tempfile.TemporaryDirectory(prefix="aios-process-qualification-") as temporary:
        work = Path(temporary)
        environment = {"HOME":os.environ["HOME"],"PATH":"/run/current-system/sw/bin","LANG":"C.UTF-8",
            "CARGO_TARGET_DIR":str(work / "cargo-target"),"CARGO_HOME":str(work / "cargo-home"),
            "XDG_CACHE_HOME":str(work / "cache"),"TMPDIR":str(work)}
        result = subprocess.run(argv, cwd=release, env=environment, capture_output=True, timeout=600, check=False)
    if snapshot.identity() != identity:
        raise ValueError("runtime target changed during installed process check")
    if len(result.stdout) + len(result.stderr) > 1024 * 1024:
        raise ValueError("installed process output exceeds bound")
    print(result.stdout.decode(errors="replace"), flush=True)
    print(result.stderr.decode(errors="replace"), flush=True)
    print("AIOS_INSTALLED_PROCESS_COMMAND " + json.dumps({"argv":argv,"upstream_exit":result.returncode,
        "target_identity":identity,"uid":os.getuid(),"caller_session_held_open":True},sort_keys=True),flush=True)
    if result.returncode or b"1 passed; 0 failed; 0 ignored" not in result.stdout:
        raise RuntimeError("installed process qualification failed")


if __name__ == "__main__":
    main()
