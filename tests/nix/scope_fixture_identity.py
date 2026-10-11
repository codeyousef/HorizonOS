"""Fixed synthetic driver target gate, never a product/admin transport API."""
import json
import os
from pathlib import Path
import subprocess
import sys


def observe():
    assert os.geteuid() == 0
    marker = json.loads(Path('/etc/aios/scope-qualification.json').read_text())
    os_release = Path('/etc/os-release').read_text()
    assert '\nID=nixos\n' in '\n' + os_release or '\nID="nixos"\n' in '\n' + os_release
    assert marker['guest_role'] == 'development'
    assert marker['management_channel'] == 'nixos-test-driver'
    virtual = subprocess.run(['/run/current-system/sw/bin/systemd-detect-virt', '--vm'], capture_output=True, check=True).stdout.strip()
    assert virtual in (b'qemu', b'kvm')
    dmi = Path('/sys/class/dmi/id/product_uuid').read_text().strip().lower()
    assert dmi == marker['dmi_uuid']
    serial = Path('/sys/block/vda/serial').read_text().strip()
    assert serial == 'root'
    key = subprocess.run(['/run/current-system/sw/bin/ssh-keygen', '-lf', '/etc/ssh/ssh_host_ed25519_key.pub', '-E', 'sha256'], capture_output=True, check=True).stdout.decode().split()[1]
    return dict(schema_version=1, dmi_uuid=dmi, installation_uuid=marker['installation_uuid'],
                boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                machine_id=Path('/etc/machine-id').read_text().strip(), guest_role=marker['guest_role'],
                management_channel=marker['management_channel'], disk_serial=serial,
                host_key_fingerprint=key, current_system=str(Path('/run/current-system').resolve(strict=True)))


if __name__ == '__main__':
    record = Path('/run/aios-scope-qualification/identity.json')
    actual = observe()
    if sys.argv[1:] == ['record']:
        assert not record.exists()
        record.parent.mkdir(mode=0o755)
        fd = os.open(record, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o644)
        with os.fdopen(fd, 'w') as stream:
            json.dump(actual, stream, sort_keys=True)
            stream.flush()
            os.fsync(stream.fileno())
    else:
        assert sys.argv[1:] == ['verify']
        assert json.loads(record.read_text()) == actual
    print(json.dumps(actual, sort_keys=True))
