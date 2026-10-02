#!/usr/bin/env python3
"""Actual Nix-packaged CPU model probe; no conversion or model download."""
import json
import hashlib
import os
from pathlib import Path
import pwd
import re
import subprocess


def main():
    release = Path(__file__).resolve().parents[2]
    source = json.loads((release / 'models/source-lock.json').read_text())
    if source['repository'] != 'Qwen/Qwen3.5-2B' or source['revision'] != '15852e8c16360a2fea060d615a32b45270f8a8fc':
        raise RuntimeError('unexpected registered model')
    built = subprocess.run(['nix','build','--json','--no-link','--no-update-lock-file','--no-write-lock-file','path:'+str(release)+'#aios-model'],
                           check=True,stdout=subprocess.PIPE,timeout=900)
    outputs = json.loads(built.stdout)
    if len(outputs) != 1:
        raise RuntimeError('unexpected inference package outputs')
    root = Path(outputs[0]['outputs']['out'])
    if not re.fullmatch(r'/nix/store/[a-z0-9]{32}-[A-Za-z0-9._+-]+',str(root)) or root.resolve() != root:
        raise RuntimeError('inference package identity mismatch')
    closure = json.loads(subprocess.check_output(['nix','path-info','--json','--recursive',str(root)],timeout=60))
    paths = sorted(closure if isinstance(closure,dict) else (entry['path'] for entry in closure))
    forbidden = re.compile(r'-(?:python[0-9.]*|torch|pytorch|cuda[^/]*|cudatoolkit|rocm[^/]*|vulkan-loader|opencl[^/]*)-')
    if any(forbidden.search(path) for path in paths):
        raise RuntimeError('forbidden inference runtime dependency')
    bridges = [Path(path) for path in paths if re.search(r'-horizon-os-llama-bridge-',path)]
    if len(bridges) != 1:
        raise RuntimeError('unexpected native runtime identity')
    native = bridges[0] / 'lib/libaios-llama-bridge.so'
    cpu_variants = sorted(path.name for path in (bridges[0] / 'bin').glob('libggml-cpu-*.so'))
    if not cpu_variants:
        raise RuntimeError('portable CPU variants missing')
    print('AIOS_MODEL_CLOSURE='+json.dumps({'paths':paths,'cpu_variants':cpu_variants,
          'bridge_sha256':hashlib.sha256(native.read_bytes()).hexdigest(),
          'probe_sha256':hashlib.sha256((root / 'bin/aios-model-probe').read_bytes()).hexdigest(),
          'no_python_torch_gpu_dependencies':True}),flush=True)
    directory = Path(pwd.getpwuid(os.geteuid()).pw_dir) / '.aios-models' / ('qwen3.5-2b-'+source['revision'])
    print('AIOS_MODEL_PACKAGE='+json.dumps({'output':str(root),'artifact_directory':str(directory)}),flush=True)
    subprocess.run([str(root/'bin/aios-model-probe'),'--qualification-artifact',str(directory)],check=True,timeout=180)


if __name__ == '__main__':
    main()
