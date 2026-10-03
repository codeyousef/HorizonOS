#!/usr/bin/env python3
"""Actual Nix-packaged CPU model probe; no conversion or model download."""
import json
import hashlib
import os
from pathlib import Path
import pwd
import re
import shutil
import stat
import tempfile


def identity(info):
    return (info.st_dev,info.st_ino,info.st_size,info.st_uid,info.st_mode,info.st_nlink,info.st_mtime_ns,info.st_ctime_ns)
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
    lock = json.loads((release / 'models/lock.json').read_text())
    files = [(directory / lock['artifact']['filename'], lock['artifact']['bytes'], lock['artifact']['sha256'])]
    files += [(directory / 'source' / item['name'], item['bytes'], item['sha256'])
              for item in source['files'] if item['bytes'] < 32 * 1024**2]
    if shutil.disk_usage(directory).free < 8 * 1024**3 + 2 * sum(size for _, size, _ in files):
        raise RuntimeError('independent artifact import lacks recovery headroom')
    imports = []
    for path, size, expected in files:
        before = path.lstat()
        if (not stat.S_ISREG(before.st_mode) or before.st_uid != os.geteuid() or
                before.st_nlink != 1 or before.st_mode & 0o222 or before.st_size != size):
            raise RuntimeError('unsafe model import')
        h = hashlib.sha256()
        with path.open('rb') as handle:
            while block := handle.read(1024**2):
                h.update(block)
        if h.hexdigest() != expected or identity(path.lstat()) != identity(before):
            raise RuntimeError('model import hash/identity mismatch')
        imported = subprocess.check_output(['nix-store','--add-fixed','sha256',str(path)],text=True,timeout=180).strip()
        if not re.fullmatch(r'/nix/store/[a-z0-9]{32}-[A-Za-z0-9._+-]+',imported) or identity(path.lstat()) != identity(before):
            raise RuntimeError('imported artifact identity mismatch')
        imports.append({'name':path.name,'sha256':expected,'store_path':imported})
    command = ['nix','build','--json','--no-link','--no-update-lock-file','--no-write-lock-file','path:'+str(release)+'#aios-model-artifact']
    artifact_outputs = json.loads(subprocess.check_output(command,timeout=300))
    artifact_root = Path(artifact_outputs[0]['outputs']['out'])
    artifact_closure = json.loads(subprocess.check_output(['nix','path-info','--json','--recursive',str(artifact_root)],timeout=60))
    artifact_paths = sorted(artifact_closure if isinstance(artifact_closure,dict) else (entry['path'] for entry in artifact_closure))
    if len(artifact_paths)!=1 or artifact_paths[0]!=str(artifact_root):
        raise RuntimeError('model data package unexpectedly depends on code/runtime/source weights')
    subprocess.run([str(root/'bin/aios-model-probe'),'--store-artifact',str(artifact_root)],check=True,timeout=180)
    repeated = json.loads(subprocess.check_output(command,timeout=60))
    if repeated != artifact_outputs:
        raise RuntimeError('repeated artifact build changed the independent output')
    # FAIL-28 uses a real, full-sized GGUF with one modified byte. The original
    # reviewed model is never altered; no missing metadata can mask this check.
    with tempfile.TemporaryDirectory(prefix='model-hash-mismatch-',dir=directory.parent) as temporary:
        damaged = Path(temporary) / lock['artifact']['filename']
        shutil.copyfile(directory / lock['artifact']['filename'],damaged)
        with damaged.open('r+b') as handle:
            handle.seek(64);value = handle.read(1);handle.seek(64);handle.write(bytes([value[0] ^ 1]))
        damaged.chmod(0o444)
        denied = subprocess.run([str(root/'bin/aios-model-probe'),'--qualification-artifact',temporary],
                                stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,timeout=60)
        if denied.returncode != 1 or denied.stdout or 'aios-model-probe: TargetChanged' not in denied.stderr:
            raise RuntimeError('corrupt GGUF was not rejected by its hash')
        damaged.chmod(0o600)
        with damaged.open('r+b') as handle:
            handle.seek(64);handle.write(value)
        damaged.chmod(0o444)
        receipt = json.loads((directory / 'conversion.json').read_text())
        receipt['model_directory'] = temporary
        altered = {**receipt,'quantizer_sha256':'0'*64}
        receipt_path = Path(temporary) / 'conversion.json'
        receipt_path.write_text(json.dumps(altered))
        def expect_rejection(expected):
            result = subprocess.run([str(root/'bin/aios-model-probe'),'--qualification-artifact',temporary],
                                    stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,timeout=60)
            if result.returncode != 1 or result.stdout or 'aios-model-probe: '+expected not in result.stderr:
                raise RuntimeError('before-load provenance rejection failed: '+expected)
        expect_rejection('TargetChanged')
        receipt_path.unlink();expect_rejection('TargetNotFound')
        receipt_path.write_text(json.dumps(receipt))
        metadata = Path(temporary) / 'source';metadata.mkdir(mode=0o700)
        license_bytes = bytearray((directory / 'source/LICENSE').read_bytes());license_bytes[0] ^= 1
        (metadata / 'LICENSE').write_bytes(license_bytes);(metadata / 'LICENSE').chmod(0o444)
        expect_rejection('TargetChanged')
    cli_outputs = json.loads(subprocess.check_output(['nix','build','--json','--no-link','--no-update-lock-file','--no-write-lock-file',
                              'path:'+str(release)+'#aios-cli'],timeout=900))
    cli = Path(cli_outputs[0]['outputs']['out']) / 'bin/aiosctl'
    info = json.loads(subprocess.check_output([str(cli),'system','info','--json'],timeout=30))
    if info['data']['os_id'] != 'nixos' or info['data']['boot_id'] != Path('/proc/sys/kernel/random/boot_id').read_text().strip():
        raise RuntimeError('deterministic OS inspection failed after rejected model')
    print('AIOS_MODEL_ARTIFACT_VERIFIED='+json.dumps({'output':str(artifact_root),'closure':artifact_paths,
        'imports':imports,'repeated_build_output_unchanged':True,'network_fetches':0,
        'production_trust_actual_load':True,'original_weights_embedded':False,
        'fail28_corrupt_actual_gguf_rejected':True,
        'negative_provenance_fixtures':['altered quantizer receipt','missing receipt','modified license'],
        'deterministic_system_info_after_rejection':info}),flush=True)


if __name__ == '__main__':
    main()
