#!/usr/bin/env python3
"""Run the actual Nix-packaged product binaries in the verified guest job."""
import json
from pathlib import Path
import re
import subprocess
import tempfile
import time


def products():
    release = Path(__file__).resolve().parents[2]
    arguments = ["nix", "build", "--json", "--no-link", "--no-update-lock-file", "--no-write-lock-file",
                 "path:" + str(release) + "#aios-core", "path:" + str(release) + "#aios-cli"]
    print("PACKAGED_BUILD=" + json.dumps(arguments), flush=True)
    built = subprocess.run(arguments, check=True, stdout=subprocess.PIPE, timeout=900)
    outputs = json.loads(built.stdout)
    if not isinstance(outputs, list) or len(outputs) != 2:
        raise ValueError("unexpected product outputs")
    paths = [Path(output["outputs"]["out"]) for output in outputs]
    for path in paths:
        if not re.fullmatch(r"/nix/store/[a-z0-9]{32}-[A-Za-z0-9._+-]+", str(path)) or path.resolve() != path:
            raise ValueError("unsafe product output")
    binaries = {}
    for program in ("aios-sessiond", "aiosctl"):
        matches = [path / "bin" / program for path in paths if (path / "bin" / program).is_file()]
        if len(matches) != 1:
            raise ValueError("missing or ambiguous product binary")
        binaries[program] = matches[0]
    return paths, binaries


def main():
    paths, binaries = products()
    boot = Path("/proc/sys/kernel/random/boot_id").read_text().strip()
    # Guest /tmp lives inside the project-owned qcow2; short paths are necessary
    # for Unix sockets. The host never runs this script or mounts this directory.
    with tempfile.TemporaryDirectory(prefix="aios-package-", dir="/tmp") as directory:
        socket = Path(directory) / "session.sock"
        child = subprocess.Popen([str(binaries["aios-sessiond"]), "--socket", str(socket)], stdin=subprocess.DEVNULL)
        try:
            deadline = time.monotonic() + 5
            while not socket.exists():
                if child.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError("packaged daemon did not bind")
                time.sleep(0.01)
            result = subprocess.run([str(binaries["aiosctl"]), "inspect", "service", "sshd.service", "--json", "--socket", str(socket)],
                                    check=True, stdout=subprocess.PIPE, timeout=10)
            value = json.loads(result.stdout)
            if value["status"] != "ok" or value["data"]["boot_id"] != boot or value["data"]["unit_name"] != "sshd.service" or value["data"]["active_state"] != "active":
                raise RuntimeError("packaged service result failed real guest verification")
            data = value["data"]
            if (data["ordering_is_not_causation"] is not True or not re.fullmatch(r"[0-9a-f]{32}",data["invocation_id"])
                    or data["ordering_after"] != sorted(set(data["ordering_after"])) or len(data["ordering_after"]) > 128):
                raise RuntimeError("service ordering/invocation observation is incomplete")
            print("AIOS_PACKAGED_SERVICE=" + json.dumps({"outputs": [str(p) for p in paths], "result": value}), flush=True)
        finally:
            if child.poll() is None:
                child.terminate()
            child.wait(timeout=5)


if __name__ == "__main__":
    main()
