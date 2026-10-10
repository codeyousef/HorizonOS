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
PATH = "/org/aios/Files1"
INTERFACE = "org.aios.Files1"


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

    def call(method, request=None, *, succeeds=True):
        command = ["busctl", "--user", "--json=short", "call", NAME, PATH, INTERFACE, method]
        if request is not None:
            command += ["s", json.dumps(request, separators=(",", ":"))]
        result = subprocess.run(command, env=env, capture_output=True, timeout=10)
        if succeeds != (result.returncode == 0):
            raise RuntimeError(f"unexpected Files1 result for {method}: {result.stderr.decode(errors='replace')[:512]}")
        if not succeeds:
            return result.stderr.decode(errors="replace")
        envelope = json.loads(result.stdout)
        if envelope.get("type") != "s" or not isinstance(envelope.get("data"), list) or len(envelope["data"]) != 1:
            raise RuntimeError("invalid busctl Files1 response envelope")
        return json.loads(envelope["data"][0])

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
        proposal = call("ProposeRoots")
        roots = proposal["data"]["roots"]
        selected = next((item for item in roots if item["display_path"] == str(documents)), None)
        if selected is None or any(not item["display_path"].startswith(str(home) + "/") for item in roots):
            raise RuntimeError("installed root proposal escaped fixed XDG roots")
        pregrant = {"schema_version": 1, "root_id": selected["root_id"], "relative_path": filename, "access": "content"}
        call("OpenScoped", pregrant, succeeds=False)
        enrolled = call("EnrollRoots", {"schema_version": 1, "proposal_id": proposal["data"]["proposal_id"],
            "approved_root_ids": [selected["root_id"]], "allowed_access": ["content"], "confirmed": True})
        if enrolled["data"]["roots"][0]["root_id"] != selected["root_id"]:
            raise RuntimeError("installed root enrollment identity changed")
        call("OpenScoped", {**pregrant, "relative_path": linkname}, succeeds=False)
        opened = call("OpenScoped", pregrant)
        handle_id = opened["data"]["file_handle"]
        read = call("ReadScoped", {"schema_version": 1, "file_handle": handle_id, "max_bytes": 128})
        if read["data"]["content"] != "consented installed content" or read["data"]["bytes_read"] != 27:
            raise RuntimeError("installed scoped content mismatch")
        revoked = call("RevokeRoot", {"schema_version": 1, "root_id": selected["root_id"], "confirmed": True})
        if not revoked["data"]["access_blocked"] or revoked["data"]["handles_revoked"] != 1:
            raise RuntimeError("installed revocation receipt is incomplete")
        call("ReadScoped", {"schema_version": 1, "file_handle": handle_id, "max_bytes": 128}, succeeds=False)
        print("AIOS_INSTALLED_FILE_SCOPES=" + json.dumps({"installed_executable": str(binary), "proposal_roots": len(roots),
            "preconsent_denied": True, "symlink_denied": True, "content_verified": True, "revocation_blocked": True}, sort_keys=True))
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
