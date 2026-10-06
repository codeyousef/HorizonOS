#!/usr/bin/env python3
"""Fixed installed journal probe retaining its authenticated SSH login."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import snapshot


def sandbox():
    argv = ["/run/current-system/sw/bin/systemctl", "show", "aios-execd.service",
        "--property=MainPID,ProtectHome,ProtectSystem,NoNewPrivileges,PrivateNetwork,BindReadOnlyPaths,CapabilityBoundingSet"]
    result = subprocess.run(argv, capture_output=True, timeout=10, check=False)
    if result.returncode or len(result.stdout)>65536 or len(result.stderr)>4096:
        raise RuntimeError("installed observer sandbox observation failed")
    values = dict(line.split("=",1) for line in result.stdout.decode().splitlines())
    expected = {"ProtectHome":"tmpfs", "ProtectSystem":"strict", "NoNewPrivileges":"yes", "PrivateNetwork":"yes",
        "CapabilityBoundingSet":"cap_dac_override cap_sys_ptrace"}
    if any(values.get(key)!=value for key,value in expected.items()) or int(values.get("MainPID","0"))<=1:
        raise RuntimeError("installed observer sandbox differs from the required profile")
    if values.get("BindReadOnlyPaths") not in ("/run/user", "/run/user:/run/user", "/run/user:/run/user:rbind"):
        raise RuntimeError("installed observer requires the fixed read-only runtime parent binding")
    status = Path("/proc") / values["MainPID"] / "status"
    capabilities = dict(line.split(":",1) for line in status.read_text().splitlines() if line.startswith(("CapEff:","CapBnd:")))
    for name in ("CapEff","CapBnd"):
        if capabilities.get(name,"").strip() != "0000000000080002":
            raise RuntimeError("installed observer kernel capabilities differ from the required profile")
        values[name] = capabilities[name].strip()
    return values


def main():
    identity = snapshot.identity()
    before = sandbox()
    release = Path(__file__).resolve().parents[2]
    argv = ["nix", "develop", "--no-update-lock-file", "--no-write-lock-file", "path:" + str(release),
        "--command", "cargo", "test", "--locked", "-p", "aios-exec", "--test", "journal_observer",
        "--", "--ignored", "--nocapture"]
    # The host publishes immutable source; generated files stay in the VM.
    # Keep this foreground process alive until the real caller has finished.
    with tempfile.TemporaryDirectory(prefix="aios-journal-qualification-") as temporary:
        work = Path(temporary)
        environment = {"HOME":os.environ["HOME"],"PATH":"/run/current-system/sw/bin","LANG":"C.UTF-8","RUST_BACKTRACE":"1",
            "CARGO_TARGET_DIR":str(work / "cargo-target"),"CARGO_HOME":str(work / "cargo-home"),
            "XDG_CACHE_HOME":str(work / "cache"),"TMPDIR":str(work)}
        result = subprocess.run(argv, cwd=release, env=environment, capture_output=True, timeout=600, check=False)
    if snapshot.identity() != identity:
        raise ValueError("runtime target changed during installed journal check")
    if len(result.stdout) + len(result.stderr) > 1024 * 1024:
        raise ValueError("installed journal output exceeds bound")
    print(result.stdout.decode(errors="replace"), flush=True)
    print(result.stderr.decode(errors="replace"), flush=True)
    print("AIOS_INSTALLED_JOURNAL_COMMAND " + json.dumps({"argv":argv,"upstream_exit":result.returncode,
        "target_identity":identity,"uid":os.getuid(),"caller_session_held_open":True},sort_keys=True),flush=True)
    after = sandbox()
    if after != before:
        raise RuntimeError("installed observer changed during journal qualification")
    print("AIOS_INSTALLED_JOURNAL_SANDBOX=" + json.dumps({"before":before,"after":after,
        "empty_homes_and_readonly_runtime_binding_verified":True}),flush=True)
    if result.returncode or b"1 passed; 0 failed; 0 ignored" not in result.stdout:
        raise RuntimeError("installed journal qualification failed")


if __name__ == "__main__":
    main()
