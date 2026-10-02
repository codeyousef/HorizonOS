"""Cold, checksummed VM recovery; no SSH or host configuration changes."""
from contextlib import contextmanager
from datetime import datetime, timezone
import errno
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import uuid

from . import guest, provision, vm
from .config import invalid, project_path, read_json
from .errors import ExitCode

FIELDS = ("disk_image", "nvram_file")
NAMES = {"disk_image": "root.qcow2", "nvram_file": "OVMF_VARS.fd"}


def location(config, name):
    if not isinstance(name, str) or not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}", name):
        raise invalid("Snapshot name must contain 1..64 letters, digits, underscores or hyphens")
    if not config.root.is_relative_to(Path('/mnt/Storage')):
        raise invalid("VM recovery storage must be under /mnt/Storage")
    return project_path(config.root, '.local/vm/snapshots/' + name + '/manifest.json', '.local/vm').parent


def binding(config):
    record = vm.load_record(config)
    trust = guest.load_trust(config)
    path = config.root / '.local/enrollment.json'
    guest.private_file(path)
    baseline = read_json(path)
    if baseline.get('schema_version') != 1 or baseline.get('host_key_fingerprint') != trust['host_key_fingerprint']:
        raise invalid('Recovery requires matching enrolled console-rooted trust')
    identity = guest.verify_identity(baseline['identity'], trust['expected'], mutation=True)
    if identity['dmi_uuid'] != record['plan']['guest_uuid'] or identity['installation_uuid'] != record['plan']['installation_uuid']:
        raise invalid('Recovery enrollment differs from the owned virtual disk')
    return record, {'guest_uuid': identity['dmi_uuid'], 'installation_uuid': identity['installation_uuid'],
                    'guest_role': identity['guest_role'], 'host_key_fingerprint': trust['host_key_fingerprint'],
                    'disk_serial': identity['disk_serial'], 'management_channel': identity['management_channel']}


def cold(config):
    state = config.root / '.local/vm/process.json'
    shutdown = None
    if state.exists():
        _, shutdown = vm.stop(config, graceful=True)
    for path in (state, config.root / '.local/vm/qemu.pid', config.paths['qmp_socket'], config.paths['serial_socket']):
        if path.exists() or path.is_symlink():
            raise invalid('Cold recovery requires cleared verified process/control artifacts')
    return shutdown


@contextmanager
def unused_files(paths):
    """Kernel write leases prove no other descriptor holds either image.

    Fail closed if unsupported or occupied; abort if any new open requests a
    lease break. Sparse copies and hashes use these same leased descriptors.
    """
    descriptors = {}
    leased = set()
    broken = [False]
    old_handler = signal.signal(signal.SIGIO, lambda *_: broken.__setitem__(0, True))
    def check():
        if broken[0] or any(fcntl.fcntl(fd, fcntl.F_GETLEASE) != fcntl.F_WRLCK for fd in descriptors.values()):
            raise provision.failure(ExitCode.VERIFICATION_FAILURE, 'IMAGE_LEASE_BROKEN', 'An image was opened during cold recovery; operation aborted')
        for path, fd in descriptors.items():
            a, b = path.lstat(), os.fstat(fd)
            if (a.st_dev, a.st_ino) != (b.st_dev, b.st_ino):
                raise invalid('Image path changed during cold recovery')
    try:
        for path in paths:
            guest.private_file(path)
            fd = os.open(path, os.O_RDWR | os.O_NOFOLLOW | os.O_NONBLOCK)
            descriptors[path] = fd
            if os.fstat(fd).st_ino != path.lstat().st_ino:
                raise invalid('Image changed before lease acquisition')
            fcntl.fcntl(fd, fcntl.F_SETOWN, os.getpid())
            try:
                fcntl.fcntl(fd, fcntl.F_SETLEASE, fcntl.F_WRLCK)
                leased.add(fd)
            except OSError as error:
                raise provision.failure(ExitCode.VERIFICATION_FAILURE, 'IMAGE_NOT_UNUSED', 'Exclusive image lease unavailable; no copying or restore performed') from error
        check()
        yield descriptors, check
        check()
    finally:
        for fd in descriptors.values():
            try:
                if fd in leased:
                    fcntl.fcntl(fd, fcntl.F_SETLEASE, fcntl.F_UNLCK)
            finally:
                os.close(fd)
        signal.signal(signal.SIGIO, old_handler)


def fd_digest(fd, check):
    h = hashlib.sha256(); offset = 0
    while True:
        check()
        data = os.pread(fd, 4 * 1024 * 1024, offset)
        if not data:
            break
        h.update(data); offset += len(data)
    return h.hexdigest()


def sparse_copy(source, target, check):
    size = os.fstat(source).st_size
    os.ftruncate(target, 0)
    offset = 0
    while offset < size:
        check()
        try:
            begin = os.lseek(source, offset, os.SEEK_DATA)
            end = min(os.lseek(source, begin, os.SEEK_HOLE), size)
        except OSError as error:
            if error.errno == errno.ENXIO:
                break
            raise
        while begin < end:
            check()
            data = os.pread(source, min(4 * 1024 * 1024, end - begin), begin)
            if not data:
                raise invalid('Image truncated during copy')
            written = 0
            while written < len(data):
                n = os.pwrite(target, data[written:], begin + written)
                if n <= 0:
                    raise invalid('Image copy did not advance')
                written += n
            begin += len(data)
        offset = end
    os.ftruncate(target, size); os.fsync(target); check()


def qcow_info(config):
    result = provision.run(['qemu-img', 'info', '--output=json', str(config.paths['disk_image'])])
    info = json.loads(result.stdout)
    if info.get('format') != 'qcow2' or info.get('backing-filename') or info.get('data-file'):
        raise invalid('Cold recovery requires a standalone qcow2 image')
    return {'format': 'qcow2', 'virtual_size': info['virtual-size']}


def snapshot(config, name):
    directory = location(config, name)
    if directory.exists():
        raise invalid('Snapshot name already exists; existing evidence is retained')
    if (config.root / '.local/vm/restore-pending.json').exists():
        raise invalid('Finish the interrupted explicit restore before taking a snapshot')
    record, target = binding(config)
    shutdown = cold(config)
    info = qcow_info(config)
    provision.private_directory(config.root, str(directory.relative_to(config.root)))
    artifacts = {}
    with unused_files([config.paths[x] for x in FIELDS]) as (fds, check):
        for field in FIELDS:
            source = fds[config.paths[field]]
            path = directory / NAMES[field]
            fd = os.open(path, os.O_RDWR | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
            try:
                sparse_copy(source, fd, check)
                digest = fd_digest(source, check)
                if fd_digest(fd, check) != digest:
                    raise invalid('Snapshot copy checksum mismatch')
                artifacts[field] = {'filename': NAMES[field], 'size': os.fstat(fd).st_size, 'sha256': digest}
            finally:
                os.close(fd)
    provision.write_json_new(directory / 'configuration.json', config.values)
    artifacts['configuration'] = {'filename': 'configuration.json', 'size': (directory / 'configuration.json').stat().st_size,
                                  'sha256': provision.digest_file(directory / 'configuration.json')}
    manifest = {'schema_version': 1, 'name': name, 'workspace': str(config.root), 'created_at': datetime.now(timezone.utc).isoformat(),
                'target': target, 'configuration': config.values, 'disk_identity': {'device': record['disk_device'], 'inode': record['disk_inode']},
                'disk': info, 'artifacts': artifacts, 'cold_proof': 'QEMU/control artifacts absent; kernel exclusive write leases held throughout copying/hashing'}
    provision.write_json_new(directory / 'manifest.json', manifest)
    index = provision.private_directory(config.root, '.local/vm/snapshot-index')
    provision.write_json_new(index / (name + '.json'), {'schema_version': 1, 'manifest_sha256': provision.digest_file(directory / 'manifest.json')})
    return ExitCode.SUCCESS, {'state': 'snapshotted', 'name': name, 'artifact_path': str(directory / 'manifest.json'),
                              'manifest_sha256': provision.digest_file(directory / 'manifest.json'), 'shutdown': shutdown,
                              'target': target, 'guest_identity_verified': False, 'cold_verified': True}


def verified_snapshot(config, name, target):
    directory = location(config, name)
    for path in (directory / 'manifest.json', config.root / '.local/vm/snapshot-index' / (name + '.json')):
        guest.private_file(path)
    index = read_json(config.root / '.local/vm/snapshot-index' / (name + '.json'))
    if set(index) != {'schema_version', 'manifest_sha256'} or index['schema_version'] != 1 or provision.digest_file(directory / 'manifest.json') != index['manifest_sha256']:
        raise invalid('Snapshot manifest failed host receipt verification')
    manifest = read_json(directory / 'manifest.json')
    fields = {'schema_version', 'name', 'workspace', 'created_at', 'target', 'configuration', 'disk_identity', 'disk', 'artifacts', 'cold_proof'}
    if set(manifest) != fields or manifest['schema_version'] != 1 or manifest['name'] != name or manifest['workspace'] != str(config.root) or manifest['target'] != target or manifest['configuration'] != config.values or set(manifest['artifacts']) != {*FIELDS, 'configuration'}:
        raise invalid('Snapshot target/configuration differs from this enrolled project VM')
    for field, item in manifest['artifacts'].items():
        expected = NAMES.get(field, 'configuration.json')
        if set(item) != {'filename', 'size', 'sha256'} or item['filename'] != expected or type(item['size']) is not int or item['size'] < 1 or not re.fullmatch('[0-9a-f]{64}', str(item['sha256'])):
            raise invalid('Invalid snapshot artifact manifest')
        path = directory / expected
        guest.private_file(path)
        if path.stat().st_size != item['size'] or provision.digest_file(path) != item['sha256']:
            raise invalid('Snapshot artifact checksum mismatch')
    if read_json(directory / 'configuration.json') != config.values:
        raise invalid('Snapshot configuration artifact mismatch')
    return directory, manifest, index['manifest_sha256']


def restore(config, name, discard):
    if not discard:
        raise provision.failure(ExitCode.AUTHORIZATION_NEEDED, 'RESTORE_DISCARD_AUTHORIZATION', 'Explicit --discard-guest-changes is required; restoring discards guest changes after the snapshot')
    record, target = binding(config)
    directory, manifest, digest = verified_snapshot(config, name, target)
    if manifest['disk_identity'] != {'device': record['disk_device'], 'inode': record['disk_inode']}:
        raise invalid('Restore destination is not the original project-owned disk')
    shutdown = cold(config)
    pending = config.root / '.local/vm/restore-pending.json'
    transaction = {'schema_version': 1, 'name': name, 'manifest_sha256': digest, 'target': target,
                   'disk_identity': manifest['disk_identity'], 'discarded_guest_changes': True}
    if pending.exists():
        guest.private_file(pending)
        if read_json(pending) != transaction:
            raise invalid('Interrupted restore must resume the same verified snapshot')
    with unused_files([config.paths[x] for x in FIELDS]) as (destinations, check):
        with unused_files([directory / NAMES[x] for x in FIELDS]) as (sources, source_check):
            def both():
                check(); source_check()
            for field in FIELDS:
                if fd_digest(sources[directory / NAMES[field]], both) != manifest['artifacts'][field]['sha256']:
                    raise invalid('Snapshot changed before restore')
            if not pending.exists():
                provision.write_json_new(pending, transaction)
            for field in FIELDS:
                fd = destinations[config.paths[field]]
                sparse_copy(sources[directory / NAMES[field]], fd, both)
                if fd_digest(fd, both) != manifest['artifacts'][field]['sha256']:
                    raise invalid('Restored image checksum mismatch; VM start remains blocked')
    # Host receipt is outside the reverted disk. Future policy integration must
    # consume this new epoch; no claim is made about unimplemented grant expiry.
    receipt_dir = provision.private_directory(config.root, '.local/vm/restores')
    epoch = str(uuid.uuid4())
    receipt = {**transaction, 'recovery_epoch': epoch, 'restored_at': datetime.now(timezone.utc).isoformat(),
               'approvals_revalidated': False, 'policy_expiry_verification': 'pending-product-policy-integration'}
    path = receipt_dir / (epoch + '.json')
    provision.write_json_new(path, receipt)
    pending.unlink()
    return ExitCode.SUCCESS, {'state': 'restored', 'name': name, 'artifact_path': str(path), 'target': target,
                              'discarded_guest_changes': True, 'checkout_and_reports_preserved': True,
                              'recovery_epoch': epoch, 'approvals_revalidated': False, 'shutdown': shutdown,
                              'guest_identity_verified': False, 'cold_verified': True}
