"""Transfer only locked public model data from an enrolled builder to a seed.

This is host administrator tooling, never a model tool or a deployment helper.
It neither evaluates code nor executes a privileged guest operation.
"""
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import shlex
import stat
import subprocess
import time

from . import guest, provision
from .config import invalid, read_json
from .errors import DevctlError, ExitCode

TARGET = 'aios-model-test'
CACHE = '.local/model-seed'
# Remote selection is limited to a store directory and the exact public files
# below. NOFOLLOW plus fstat before/after binds the stream to one immutable file.
READER = '''import os,stat,sys
p=sys.argv[1];size=int(sys.argv[2])
f=os.open(p,os.O_RDONLY|os.O_NOFOLLOW)
a=os.fstat(f)
assert stat.S_ISREG(a.st_mode) and a.st_uid==0 and a.st_mode&0o222==0 and a.st_size==size
with os.fdopen(f,'rb') as h:
 while b:=h.read(1024*1024):sys.stdout.buffer.write(b)
 z=os.fstat(h.fileno())
 assert (a.st_dev,a.st_ino,a.st_size,a.st_mtime_ns,a.st_ctime_ns)==(z.st_dev,z.st_ino,z.st_size,z.st_mtime_ns,z.st_ctime_ns)
'''


def entries(root):
    lock = read_json(root/'models/lock.json')
    source = read_json(root/'models/source-lock.json')
    if (lock['schema_version'] != 1 or lock['profile'] != 'normal' or lock['availability'] != 'available'
            or provision.digest_file(root/'models/source-lock.json') != lock['source_lock_sha256']
            or source['runtime']['revision'] != lock['runtime_revision'] or source['inference_backend'] != 'cpu'):
        raise invalid('Model seed requires the reviewed normal CPU model lock')
    values = [{'name': lock['artifact']['filename'], 'bytes': lock['artifact']['bytes'], 'sha256': lock['artifact']['sha256'], 'remote': lock['artifact']['filename']}]
    values += [{**v, 'remote': 'source/'+v['name']} for v in source['files'] if v['bytes'] < 32*1024**2]
    if len(values) > 32 or len({v['name'] for v in values}) != len(values):
        raise invalid('Invalid model seed file set')
    for v in values:
        if (not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,127}', v['name']) or not re.fullmatch(r'[0-9a-f]{64}', v['sha256'])
                or type(v['bytes']) is not int or not 0 < v['bytes'] < 2*1024**3):
            raise invalid('Invalid locked model seed file')
    return values


def file_identity(info):
    # Reading legitimately updates atime; replacement/content/permission changes
    # are represented by the stable inode fields and mtime/ctime instead.
    return (info.st_dev, info.st_ino, info.st_size, info.st_mode, info.st_uid,
            info.st_gid, info.st_nlink, info.st_mtime_ns, info.st_ctime_ns)


def validate_cache(root):
    directory = root/CACHE
    if directory.is_symlink() or directory.resolve() != directory or not directory.is_dir():
        raise invalid('Verified model seed cache is missing')
    values = entries(root)
    if {p.name for p in directory.iterdir()} != {v['name'] for v in values} | {'receipt.json'}:
        raise invalid('Model seed cache has missing or unexpected files')
    receipt = read_json(directory/'receipt.json')
    if receipt.get('files') != values or receipt.get('model_lock_sha256') != provision.digest_file(root/'models/lock.json'):
        raise invalid('Model seed receipt does not match the source lock')
    for v in values:
        path = directory/v['name']; info = path.lstat()
        if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_size != v['bytes'] or info.st_mode&0o222
                or provision.digest_file(path) != v['sha256'] or file_identity(path.lstat()) != file_identity(info)):
            raise invalid('Model seed data changed or failed its reviewed checksum')
    return values


def export(config, artifact):
    if not config.root.is_relative_to(Path('/mnt/Storage')):
        raise invalid('Model storage must be under /mnt/Storage')
    if not re.fullmatch(r'/nix/store/[a-z0-9]{32}-horizon-os-model-normal-[a-f0-9]{12}', artifact):
        raise invalid('Only the reviewed immutable model data package can be exported')
    values = entries(config.root)
    if not artifact.endswith('-'+values[0]['sha256'][:12]):
        raise invalid('Model package name does not match the reviewed artifact')
    trust, identity = guest.enrolled_identity(config)
    if (config.root/CACHE).exists():
        validate_cache(config.root)
        return ExitCode.SUCCESS, {'cache':str(config.root/CACHE), 'files':values, 'reused':True}
    directory = provision.private_directory(config.root, CACHE)
    encoded = base64.b64encode(READER.encode()).decode()
    for v in values:
        _, before = guest.enrolled_identity(config)
        if before != identity:
            raise invalid('Builder identity changed before model transfer')
        command = "python3 -I -c " + shlex.quote("import base64;exec(compile(base64.b64decode('"+encoded+"'),'<aios-model-data-reader>','exec'))")
        command += ' '+shlex.quote(artifact+'/'+v['remote'])+' '+str(v['bytes'])
        partial = directory/(v['name']+'.part')
        process = subprocess.Popen([*guest.ssh_arguments(config)[:-1], command], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        digest = hashlib.sha256(); size = 0
        try:
            with os.fdopen(os.open(partial,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600),'wb') as h, selectors.DefaultSelector() as selector:
                os.set_blocking(process.stdout.fileno(),False)
                selector.register(process.stdout,selectors.EVENT_READ)
                deadline=time.monotonic()+180
                while selector.get_map():
                    if time.monotonic() >= deadline:
                        raise DevctlError(ExitCode.TIMEOUT,'MODEL_TRANSFER_TIMEOUT','Model transfer timed out; partial cache retained')
                    for key,_ in selector.select(1):
                        b=os.read(key.fd,1024*1024)
                        if not b:
                            selector.unregister(key.fileobj); continue
                        size+=len(b)
                        if size > v['bytes']:
                            raise invalid('Model stream exceeds its reviewed size')
                        digest.update(b); h.write(b)
                h.flush(); os.fsync(h.fileno())
            if process.wait(timeout=5) != 0 or size != v['bytes'] or digest.hexdigest() != v['sha256']:
                raise invalid('Model stream failed size/checksum/remote identity validation')
        finally:
            if process.poll() is None:
                process.kill(); process.wait()
            process.stdout.close()
        if guest.enrolled_identity(config)[1] != identity:
            raise invalid('Builder identity changed after model transfer')
        partial.chmod(0o444); partial.rename(directory/v['name'])
    provision.write_json_new(directory/'receipt.json', {'schema_version':1,'files':values,'model_lock_sha256':provision.digest_file(config.root/'models/lock.json'),
        'builder_identity':identity,'host_key_fingerprint':trust['host_key_fingerprint'],'artifact':artifact})
    validate_cache(config.root)
    return ExitCode.SUCCESS, {'cache':str(directory),'files':values,'identity_verified':True,'artifact':artifact}


def stage(config, seed):
    if config.values['guest_build_target'] != TARGET:
        return
    values = validate_cache(config.root)
    target = seed/'model'; target.mkdir(mode=0o700)
    imports = []
    script = ['#!/usr/bin/env bash', 'set -euo pipefail', 'mode=${1:---verify}', '[[ $# -le 1 && ( $mode == --verify || $mode == --import ) ]]', 'cd -- "$(dirname -- "${BASH_SOURCE[0]}")/model"']
    for v in values:
        src = config.root/CACHE/v['name']; dest=target/v['name']
        # Copy bytes rather than sharing mutable cache inodes with frozen media.
        with src.open('rb') as source, os.fdopen(os.open(dest,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o444),'wb') as out:
            while block:=source.read(1024*1024):out.write(block)
        if provision.digest_file(dest) != v['sha256'] or dest.stat().st_size != v['bytes']:
            raise invalid('Model seed copy failed reviewed integrity check')
        script += [f"test \"$(stat -c %s {shlex.quote(v['name'])})\" = {v['bytes']}",
                   f"printf '%s\\n' '{v['sha256']}  {v['name']}' | sha256sum --check --strict"]
        imports.append(f"nix-store --store /mnt --add-fixed sha256 {shlex.quote(v['name'])} >/dev/null")
    provision.write_new(seed/'model-import.sh', ('\n'.join(script+['if [[ $mode == --import ]]; then', *imports, 'fi'])+'\n').encode(),0o444)


def copy_cache(source_root, target_root):
    values = validate_cache(source_root)
    directory = provision.private_directory(target_root, CACHE)
    for name in [*[v['name'] for v in values], 'receipt.json']:
        source = source_root/CACHE/name
        target = directory/name
        with source.open('rb') as inp, os.fdopen(os.open(target,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o444 if name!='receipt.json' else 0o600),'wb') as out:
            while b:=inp.read(1024*1024):out.write(b)
    validate_cache(target_root)
