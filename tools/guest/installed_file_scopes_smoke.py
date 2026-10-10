#!/usr/bin/env python3
"""Exercise installed Files1 grants against a synthetic file in the caller's Documents root."""
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import uuid


NAME = "org.aios.Session1"



def main():
    if os.geteuid() == 0:
        raise RuntimeError("installed file-scope qualification requires a normal user")
    runtime = Path(f"/run/user/{os.geteuid()}")
    env = {**os.environ, "XDG_RUNTIME_DIR": str(runtime), "DBUS_SESSION_BUS_ADDRESS": "unix:path=" + str(runtime / "bus")}
    binary = Path("/run/current-system/sw/bin/aios-sessiond").resolve(strict=True)
    info = binary.stat()
    if (not re.fullmatch(r"/nix/store/[a-z0-9]{32}-aios-core-[A-Za-z0-9._+-]+/bin/aios-sessiond", str(binary))
            or not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o222):
        raise RuntimeError("session broker is not the protected installed executable")
    subprocess.run(["systemctl", "--user", "restart", "aios-sessiond.service"], env=env, check=True, timeout=20)
    owner = subprocess.run(["busctl", "--user", "call", "org.freedesktop.DBus", "/org/freedesktop/DBus",
                            "org.freedesktop.DBus", "GetConnectionUnixProcessID", "s", NAME],
                           env=env, check=True, capture_output=True, timeout=5).stdout.decode().strip()
    match = re.fullmatch(r"u ([1-9][0-9]*)", owner)
    if not match or Path(f"/proc/{match[1]}/exe").resolve(strict=True) != binary:
        raise RuntimeError("Files1 bus owner is not the installed broker")


    home = Path.home()
    documents = home / "Documents"
    made_documents = False
    if not documents.exists():
        documents.mkdir(mode=0o700)
        made_documents = True
    root_info = documents.lstat()
    if not stat.S_ISDIR(root_info.st_mode) or documents.is_symlink() or root_info.st_uid != os.geteuid():
        raise RuntimeError("Documents fixture root is unsafe")
    token = uuid.uuid4().hex
    filename = f".aios-file-scope-{token}.txt"
    linkname = f".aios-file-scope-{token}.link"
    target = documents / filename
    link = documents / linkname
    descriptor = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(b"consented installed content")
            handle.flush()
            os.fsync(handle.fileno())
        os.symlink("/etc/passwd", link)
        release = Path(__file__).resolve().parents[2]
        # Native enrollment requires an explicitly selected own graphical
        # session. A headless client cannot invent one or claim approval.
        session = os.environ.get("XDG_SESSION_ID", "")
        if not re.fullmatch(r"[A-Za-z0-9_-]{1,128}", session):
            raise RuntimeError("installed native file-scope qualification requires an explicitly selected graphical session and native user confirmation")
        command = ["nix", "develop", "--no-update-lock-file", "--no-write-lock-file", "path:" + str(release),
            "--command", "cargo", "run", "--locked", "--quiet", "-p", "aios-session", "--example",
            "installed_file_scopes", "--", str(documents), filename, linkname, session]
        result = subprocess.run(command, env=env, capture_output=True, timeout=300)
        marker = "AIOS_INSTALLED_FILE_SCOPES_CLIENT="
        lines = result.stdout.decode(errors="replace").splitlines()
        if result.returncode or not any(line.startswith(marker) for line in lines):
            raise RuntimeError("installed Files1 client failed: " + result.stderr.decode(errors="replace")[-1024:])
        proof = json.loads(next(line[len(marker):] for line in lines if line.startswith(marker)))
        if set(proof) != {"preconsent_denied", "symlink_denied", "content_verified", "revocation_blocked"} or not all(proof.values()):
            raise RuntimeError("installed Files1 client proof is incomplete")
        print("AIOS_INSTALLED_FILE_SCOPES=" + json.dumps({"installed_executable": str(binary), **proof}, sort_keys=True))
    finally:
        link.unlink(missing_ok=True)
        target.unlink(missing_ok=True)
        subprocess.run(["systemctl", "--user", "restart", "aios-sessiond.service"], env=env, check=False, timeout=20)
        if made_documents:
            try:
                documents.rmdir()
            except OSError:
                pass


if __name__ == "__main__":
    main()
