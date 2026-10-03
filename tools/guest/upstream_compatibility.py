#!/usr/bin/env python3
"""Actual pinned package/runtime metadata inside a verified development job."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import re


def main():
    release = Path(__file__).resolve().parents[2]
    argv = ["nix", "eval", "--json", "--no-update-lock-file", "--no-write-lock-file",
            "--option", "pure-eval", "true", "--option", "allow-import-from-derivation", "false",
            "path:" + str(release) + "#lib.upstreamCompatibility"]
    result = subprocess.run(argv, capture_output=True, check=True, timeout=180)
    value = json.loads(result.stdout)
    lock = json.loads((release / "models/source-lock.json").read_text())
    if value["llama_source_nar_hash"] != lock["runtime"]["source_nar_hash"] or any(not item["available"] or not item["licenses"] for item in value["packages"]):
        raise ValueError("locked upstream metadata differs or is unavailable")
    if set(value["image_attributes"]) != {"aios-dev", "aios-desktop-test"}:
        raise ValueError("image attribute contract differs")
    runtime = Path(os.environ["AIOS_LLAMA_BRIDGE"])
    upstream = Path(os.environ["AIOS_LLAMA_SOURCE"])
    for path in (runtime, upstream):
        if not str(path).startswith("/nix/store/") or path.resolve() != path:
            raise ValueError("pinned runtime path differs")
    public = {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in {
        "converter": upstream / "convert_hf_to_gguf.py", "quantizer": runtime / "bin/llama-quantize",
        "runtime_license": runtime / "share/licenses/llama.cpp/LICENSE", "native_bridge": runtime / "lib/libaios-llama-bridge.so"}.items()}
    model_lock = json.loads((release / "models/lock.json").read_text())
    if public["converter"] != model_lock["converter_sha256"]:
        raise ValueError("converter source differs from the qualified normal artifact")
    built = subprocess.run(["nix", "build", "--json", "--no-link", "--no-update-lock-file", "--no-write-lock-file",
                            "path:" + str(release) + "#aios-model"], capture_output=True, check=True, timeout=900)
    outputs = json.loads(built.stdout)
    if len(outputs) != 1:
        raise ValueError("unexpected inference package outputs")
    package = outputs[0]["outputs"]["out"]
    closure = json.loads(subprocess.check_output(["nix", "path-info", "--json", "--recursive", package], timeout=60))
    paths = sorted(closure if isinstance(closure, dict) else (entry["path"] for entry in closure))
    forbidden = re.compile(r"-(?:python[0-9.]*|torch|pytorch|cuda[^/]*|cudatoolkit|rocm[^/]*|vulkan-loader|opencl[^/]*)-")
    if any(forbidden.search(path) for path in paths) or str(runtime) not in paths:
        raise ValueError("inference package dependency contract differs")
    first = Path("/proc/cpuinfo").read_text().split("\n\n", 1)[0]
    cpu = {key.strip(): val.strip() for key, val in (line.split(":", 1) for line in first.splitlines() if ":" in line)}
    value.update(evidence_kind="real-locked-upstream-compatibility", command=argv, upstream_exit=result.returncode,
                 lock_hashes={name: hashlib.sha256((release / name).read_bytes()).hexdigest() for name in ("flake.lock", "Cargo.lock")},
                 runtime_paths={"bridge": str(runtime), "source": str(upstream)}, artifact_hashes=public,
                 model_source_license=lock["license"], runtime_license=lock["runtime"]["license"],
                 original_normal_conversion_hashes={"converter": model_lock["converter_sha256"], "quantizer": model_lock["quantizer_sha256"]},
                 inference_closure={"package": package, "paths": paths, "no_python_torch_gpu_dependencies": True,
                                    "cpu_variants": sorted(path.name for path in (runtime / "bin").glob("libggml-cpu-*.so"))},
                 cpu={"architecture": os.uname().machine, "logical_cpus": os.cpu_count(),
                      "flags": sorted(cpu.get("flags", "").strip().split())},
                 quality_qualified=False, performance_qualified=False,
                 limitations=["Metadata availability is distinct from actual installed capabilities; build/boot artifacts are linked separately.",
                              "CPU flags are observations of this host-launched KVM guest, not qualified performance benchmarks.",
                              "Original normal conversion keeps its recorded quantizer hash; current package rebuild hashes are recorded separately."])
    print("AIOS_UPSTREAM_COMPATIBILITY=" + json.dumps(value, sort_keys=True), flush=True)


if __name__ == "__main__":
    main()
