#!/usr/bin/env python3
"""Separate development artifact fetch/conversion; never part of inference."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pwd
import shutil
import stat
import subprocess
import sys
import urllib.request

PROFILES = {"normal": ("Qwen/Qwen3.5-2B", "15852e8c16360a2fea060d615a32b45270f8a8fc"),
            "low": ("Qwen/Qwen3.5-0.8B", "2fc06364715b967f1860aea9cf38778875588b17"),
            "high": ("Qwen/Qwen3.5-4B", "851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a")}


def source_lock(release, profile):
    if profile not in PROFILES:
        raise ValueError("unregistered model profile")
    path = release / ("models/source-lock.json" if profile == "normal" else "models/source-locks/" + profile + ".json")
    lock = json.loads(path.read_text())
    if (lock["repository"], lock["revision"]) != PROFILES[profile] or lock["runtime"]["revision"] != "b64739ea393b3c9d07cc9907e0a611f707838051" or lock["inference_backend"] != "cpu":
        raise RuntimeError("unexpected registered model/runtime")
    return lock


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as f:
        while block := f.read(1024 * 1024):
            h.update(block)
    return h.hexdigest()


def private(path):
    path.mkdir(mode=0o700, exist_ok=True)
    info = path.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.geteuid() or info.st_mode & 0o077:
        raise RuntimeError("unsafe model development directory")
    return path


def fetch(destination, expected, url):
    if destination.exists():
        info = destination.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_nlink != 1:
            raise RuntimeError("unsafe existing source artifact")
        if info.st_size == expected["bytes"] and digest(destination) == expected["sha256"]:
            return
        raise RuntimeError("existing source artifact hash mismatch")
    partial = destination.with_name(destination.name + ".part")
    fd = os.open(partial, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "r+b") as file:
        info = os.fstat(file.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_nlink != 1 or info.st_size > expected["bytes"]:
            raise RuntimeError("unsafe partial source artifact")
        offset = info.st_size
        if offset == expected["bytes"]:
            if digest(partial) != expected["sha256"]:
                raise RuntimeError("complete partial source artifact hash mismatch")
            partial.chmod(0o444);partial.rename(destination)
            return
        request = urllib.request.Request(url, headers={"Range":f"bytes={offset}-"} if offset else {})
        with urllib.request.urlopen(request, timeout=60) as response:
            if response.status == 206:
                if response.headers.get("Content-Range", "").split("-")[0] != f"bytes {offset}":
                    raise RuntimeError("unexpected source range")
                file.seek(offset)
            elif response.status == 200:
                file.seek(0);file.truncate();offset = 0
            else:
                raise RuntimeError("unexpected source status")
            reported = offset
            while block := response.read(1024 * 1024):
                offset += len(block)
                if offset > expected["bytes"]:
                    raise RuntimeError("source artifact exceeds pinned size")
                file.write(block)
                if offset - reported >= 256 * 1024 * 1024:
                    print("AIOS_SOURCE_PROGRESS=" + json.dumps({"artifact":destination.name,"received_bytes":offset,"expected_bytes":expected["bytes"]}),flush=True)
                    reported = offset
        file.flush();os.fsync(file.fileno())
    if partial.stat().st_size != expected["bytes"] or digest(partial) != expected["sha256"]:
        raise RuntimeError("source artifact hash mismatch")
    partial.chmod(0o444);partial.rename(destination)


def convert(release, profile="normal"):
    if os.geteuid() == 0 or 'ID=nixos' not in Path('/etc/os-release').read_text():
        raise RuntimeError("conversion requires the non-root verified NixOS development job")
    lock = source_lock(release, profile)
    home = Path(pwd.getpwuid(os.geteuid()).pw_dir)
    store = private(home / ".aios-models")
    directory = private(store / (lock["repository"].split("/")[1].lower() + "-" + lock["revision"]))
    descriptor = os.open(directory / "conversion.lock", os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW,0o600)
    with os.fdopen(descriptor,"r+") as lockfile:
        fcntl.flock(lockfile,fcntl.LOCK_EX)
        source = private(directory / "source")
        reserve = max(15 * 1024**3, sum(item["bytes"] for item in lock["files"]) * 3 + 8 * 1024**3)
        if shutil.disk_usage(directory).free < reserve:
            raise RuntimeError("conversion space reserve unavailable")
        for artifact in lock["files"]:
            name = artifact["name"]
            if Path(name).name != name:
                raise RuntimeError("invalid registered source filename")
            url = "https://huggingface.co/" + lock["repository"] + "/resolve/" + lock["revision"] + "/" + name
            fetch(source / name,artifact,url)
        upstream = Path(os.environ["AIOS_LLAMA_SOURCE"])
        runtime = Path(os.environ["AIOS_LLAMA_BRIDGE"])
        for path in (upstream,runtime):
            if not str(path).startswith('/nix/store/') or path.resolve() != path:
                raise RuntimeError("runtime build identity mismatch")
        receipt_path = directory / "conversion.json"
        if receipt_path.exists():
            info = receipt_path.lstat()
            if not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_nlink != 1:
                raise RuntimeError("unsafe existing conversion receipt")
            receipt = json.loads(receipt_path.read_text())
            output = directory / "model-q4_k_m.gguf"
            info = output.lstat()
            if (receipt.get("source") != lock or receipt.get("profile", "normal") != profile
                    or receipt.get("runtime_path") != str(runtime)
                    or receipt.get("converter_sha256") != digest(upstream / "convert_hf_to_gguf.py")
                    or receipt.get("quantizer_sha256") != digest(runtime / "bin/llama-quantize")
                    or not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid() or info.st_nlink != 1
                    or info.st_mode & 0o222 or info.st_size != receipt["artifact"]["bytes"]
                    or digest(output) != receipt["artifact"]["sha256"]):
                raise RuntimeError("existing converted artifact identity mismatch")
            print("AIOS_MODEL_CONVERSION=" + json.dumps(receipt), flush=True)
            return receipt
        full = directory / "model-f16.gguf"
        output = directory / "model-q4_k_m.gguf"
        env = {**os.environ,"HF_HUB_OFFLINE":"1","TRANSFORMERS_OFFLINE":"1","OMP_NUM_THREADS":"4"}
        # No remote model code, third-party GGUF or GPU conversion is accepted.
        subprocess.run(["python3",str(upstream / "convert_hf_to_gguf.py"),str(source),"--outfile",str(full),"--outtype","f16"],env=env,check=True,timeout=900)
        subprocess.run([str(runtime / "bin/llama-quantize"),str(full),str(output),"Q4_K_M","4"],env=env,check=True,timeout=900)
        output.chmod(0o444)
        receipt={"schema_version":1,"profile":profile,"evidence_kind":"real-official-weight-conversion","source":lock,
                 "artifact":{"filename":output.name,"bytes":output.stat().st_size,"sha256":digest(output)},
                 "runtime_path":str(runtime),"converter_sha256":digest(upstream / "convert_hf_to_gguf.py"),
                 "quantizer_sha256":digest(runtime / "bin/llama-quantize"),
                 "chat_template_sha256":digest(source / "chat_template.jinja"),
                 "versions":subprocess.check_output(["python3","-c","import torch,transformers,numpy; print(torch.__version__,transformers.__version__,numpy.__version__)"],text=True).strip(),
                 "model_directory":str(directory),"quality_qualified":False,"performance_qualified":False}
        (directory / "conversion.json").write_text(json.dumps(receipt,indent=2)+'\n')
        print("AIOS_MODEL_CONVERSION="+json.dumps(receipt),flush=True)
        return receipt


def main():
    if len(sys.argv) > 2:
        raise ValueError("only a registered profile may be selected")
    convert(Path(__file__).resolve().parents[2], sys.argv[1] if len(sys.argv) == 2 else "normal")


if __name__ == "__main__":
    main()
