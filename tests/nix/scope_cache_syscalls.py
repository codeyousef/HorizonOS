"""Fixed synthetic startup diagnostics under the packaged broker's filters.

No file contents, environment, user-selected paths or syscall arguments are
accepted. This runs only in the optional NixOS qualification account.
"""
import ctypes
import json
import os
import pwd
import struct


assert os.geteuid() == 1000 and os.uname().machine == 'x86_64'
assert pwd.getpwuid(1000).pw_dir == '/home/alice'
libc = ctypes.CDLL(None, use_errno=True)
rows = []
for path in ('/home/alice', '/home/alice/.local', '/home/alice/.local/state'):
    how = ctypes.create_string_buffer(struct.pack('QQQ',
        os.O_PATH | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, 0, 0x02 | 0x04))
    ctypes.set_errno(0)
    opened = libc.syscall(ctypes.c_long(437), ctypes.c_long(-100),
                          ctypes.c_char_p(path.encode()), how, ctypes.c_size_t(24))
    opening = dict(openat2_status=opened, openat2_errno=ctypes.get_errno())
    if opened >= 0:
        os.close(opened)
    try:
        fd = os.open(path, os.O_PATH | os.O_DIRECTORY | os.O_NOFOLLOW)
    except OSError as error:
        rows.append(dict(path=path, open_errno=error.errno))
        continue
    try:
        try:
            metadata = os.fstat(fd)
            row = dict(path=path, uid=metadata.st_uid, mode=oct(metadata.st_mode),
                       device=metadata.st_dev, inode=metadata.st_ino)
        except OSError as error:
            row = dict(path=path, fstat_errno=error.errno)
        row.update(opening)
        # x86_64 Linux statx ABI, matching the production descriptor request.
        buffer = ctypes.create_string_buffer(256)
        ctypes.set_errno(0)
        status = libc.syscall(ctypes.c_long(332), ctypes.c_long(fd),
                              ctypes.c_char_p(b''), ctypes.c_int(0x1000 | 0x4000),
                              ctypes.c_uint(0x1000), buffer)
        row.update(statx_status=status, statx_errno=ctypes.get_errno(),
                   statx_mask=struct.unpack_from('I', buffer.raw, 0)[0],
                   mount_id=struct.unpack_from('Q', buffer.raw, 144)[0])
        rows.append(row)
    finally:
        os.close(fd)
print('CACHE_SYSCALL_DIAGNOSTIC=' + json.dumps(rows), flush=True)
