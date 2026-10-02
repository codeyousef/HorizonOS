#!/usr/bin/env python3
"""Registered root audit inside the read-only mounted installed NixOS guest.

No credential contents or password hashes are returned. Storage/DMI/virtual
device guards are checked by the official-installer wrapper before chroot.
"""
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import stat
import subprocess
import sys
import uuid

SERVICES = ("aios-state", "aios-observer", "aios-builder", "aios-model")


class Denied(Exception):
    def __init__(self, check, evidence=None):
        super().__init__(check)
        self.evidence = evidence


def require(condition, check, evidence=None):
    if not condition:
        raise Denied(check, evidence)


def key_fingerprint(value):
    words = value.split()
    require(len(words) in (2, 3) and words[0] == "ssh-ed25519", "host-public-key-format")
    try:
        raw = base64.b64decode(words[1], validate=True)
    except ValueError as error:
        raise Denied("host-public-key-encoding") from error
    require(len(raw) == 51 and raw[:19] == b"\x00\x00\x00\x0bssh-ed25519\x00\x00\x00\x20", "host-public-key-encoding")
    return "SHA256:" + base64.b64encode(hashlib.sha256(raw).digest()).decode().rstrip("=")


def account_report(passwd, groups, shadow):
    users, members, gids, states = {}, {}, {}, {}
    for line in passwd.splitlines():
        parts = line.split(":")
        require(len(parts) == 7 and parts[0] not in users, "passwd-format")
        users[parts[0]] = {"uid": int(parts[2]), "gid": int(parts[3]), "home": parts[5], "shell": parts[6]}
    for line in groups.splitlines():
        parts = line.split(":")
        require(len(parts) == 4 and parts[0] not in members, "group-format")
        members[parts[0]] = set(parts[3].split(",")) - {""}
        gids[parts[0]] = int(parts[2])
    for line in shadow.splitlines():
        parts = line.split(":")
        require(len(parts) == 9 and parts[0] not in states, "shadow-format")
        # Discard the actual hash immediately; evidence carries only a state.
        value = parts[1]
        states[parts[0]] = "locked" if value.startswith(("!", "*")) else "hashed" if re.fullmatch(r"\$[A-Za-z0-9]+\$[^\s:]+", value) else "unsafe"
    require("wheel" in members and set(("root", "dev", "tester", *SERVICES)) <= users.keys(), "required-accounts")
    require(users["root"]["uid"] == 0, "root-account-identity")
    require(states.get("dev") == "locked" and states.get("root") == "locked" and states.get("tester") == "hashed", "bootstrap-password-states")
    result = {}
    for name in ("dev", "tester", *SERVICES):
        user = users[name]
        user_groups = sorted(group for group, names in members.items() if name in names or gids[group] == user["gid"])
        require(("wheel" in user_groups) == (name == "tester"), "wheel-membership")
        if name in SERVICES:
            require(0 < user["uid"] < 1000 and user["gid"] == gids.get(name) and Path(user["shell"]).name == "nologin" and states.get(name) == "locked", "service-account-isolation")
        else:
            require(user["uid"] >= 1000 and user["home"] == "/home/" + name, "normal-user-identity")
        result[name] = {**user, "groups": user_groups, "password_state": states[name]}
    require(len({users[name]["uid"] for name in result}) == len(result), "account-uid-uniqueness")
    return result


def mount_report(fstab, installation_uuid):
    mounts = {}
    for line in fstab.splitlines():
        row = shlex.split(line, comments=True)
        if not row:
            continue
        require(len(row) == 6 and row[1] not in mounts and row[2] != "swap", "fstab-format-no-swap")
        mounts[row[1]] = {"source": row[0], "type": row[2], "options": row[3].split(",")}
    for path, subvolume in (("/", "@root"), ("/home", "@home"), ("/nix", "@nix"), ("/var", "@var")):
        item = mounts.get(path, {})
        require(item.get("type") == "btrfs" and item.get("source") in ("UUID=" + installation_uuid, "/dev/disk/by-uuid/" + installation_uuid) and any(option in ("subvol=" + subvolume, "subvol=/" + subvolume) for option in item.get("options", [])), "btrfs-subvolume-configuration")
    efi = mounts.get("/boot", {})
    require(efi.get("type") == "vfat" and {"fmask=0077", "dmask=0077"} <= set(efi.get("options", [])), "efi-permissions")
    return {path: mounts[path] for path in ("/", "/home", "/nix", "/var", "/boot")}


def nix_report(contents):
    settings = {}
    for line in contents.splitlines():
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        require("=" in line, "nix-settings-format")
        key, value = (part.strip() for part in line.split("=", 1))
        require(key not in settings, "nix-settings-duplicate")
        settings[key] = value
    trusted = settings.get("trusted-users", "").split()
    require(len(trusted) <= 32 and all(re.fullmatch(r"@?[A-Za-z_][A-Za-z0-9_-]{0,31}", user) for user in trusted), "nix-trusted-users-format")
    require(set(trusted) == {"root"}, "nix-root-only-trust", {"trusted_users": trusted})
    require(settings.get("sandbox") == "true", "nix-sandbox")
    return {"trusted_users": ["root"], "configured_trusted_users": trusted, "sandbox": True}


def credential_metadata(path, *, secret=False):
    item = Path(path)
    info = item.lstat()
    mode = stat.S_IMODE(info.st_mode)
    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and info.st_uid == 0 and not item.resolve().is_relative_to(Path("/nix/store")), "credential-owned-outside-store")
    require(mode == 0o600 if secret else not mode & 0o007, "credential-permissions")
    require(not secret or info.st_size >= 48, "tester-secret-present")
    return {"uid": info.st_uid, "gid": info.st_gid, "mode": oct(mode), "outside_nix_store": True, "regular_single_link": True}


def audit(guest_uuid, installation_uuid, fingerprint):
    require(os.geteuid() == 0, "root-auditor-required")
    require(str(uuid.UUID(guest_uuid)) == guest_uuid and str(uuid.UUID(installation_uuid)) == installation_uuid and re.fullmatch(r"SHA256:[A-Za-z0-9/+]{43}", fingerprint), "expected-target-format")
    def text(path):
        value = Path(path).read_text()
        require(len(value) <= 262144, "audit-input-size")
        return value
    release = dict(line.split("=", 1) for line in text("/etc/os-release").splitlines() if "=" in line)
    require(shlex.split(release.get("ID", "")) == ["nixos"], "installed-nixos")
    require(text("/etc/aios/installation-uuid").strip() == installation_uuid and text("/etc/aios/expected-dmi-uuid").strip() == guest_uuid and text("/etc/aios/guest-role").strip() == "development", "installed-target-role")
    require(key_fingerprint(text("/etc/ssh/ssh_host_ed25519_key.pub")) == fingerprint, "pinned-installed-host-key")
    metadata = {name: credential_metadata(path, secret=name == "tester_secret") for name, path in (("tester_secret", "/root/.aios-tester-secret"), ("shadow", "/etc/shadow"), ("host_private_key", "/etc/ssh/ssh_host_ed25519_key"))}
    require(metadata["host_private_key"]["mode"] == "0o600", "host-private-key-permissions")
    accounts = account_report(text("/etc/passwd"), text("/etc/group"), text("/etc/shadow"))
    mounts = mount_report(text("/etc/fstab"), installation_uuid)
    nix = nix_report(text("/etc/nix/nix.conf"))
    completed = subprocess.run(["/nix/var/nix/profiles/system/sw/bin/sshd", "-T"], capture_output=True, text=True, timeout=10, check=False)
    require(completed.returncode == 0 and len(completed.stdout) <= 262144, "sshd-effective-check")
    effective = dict(line.split(" ", 1) for line in completed.stdout.splitlines() if " " in line)
    expected = {"permitrootlogin": "no", "passwordauthentication": "no", "kbdinteractiveauthentication": "no", "allowusers": "dev", "allowtcpforwarding": "no", "allowstreamlocalforwarding": "no", "allowagentforwarding": "no", "x11forwarding": "no", "permittunnel": "no"}
    require(all(effective.get(key) == value for key, value in expected.items()), "sshd-isolation", {"settings": {key: effective.get(key) for key in expected}, "stdout_bytes": len(completed.stdout), "stderr_bytes": len(completed.stderr), "leading_whitespace_lines": sum(line[:1].isspace() for line in completed.stdout.splitlines()), "selected_keys_case_insensitive": [key for key in effective if key.lower() in expected]})
    return {"schema_version": 1, "guest_uuid": guest_uuid, "installation_uuid": installation_uuid, "guest_role": "development", "host_key_fingerprint": fingerprint,
            "accounts": accounts, "credential_metadata": metadata, "fstab": mounts, "nix": nix, "sshd": expected,
            "assertions": {"installed_identity": True, "dev_not_wheel_or_nix_trusted": True, "tester_wheel_with_guest_only_secret": True, "service_accounts_nologin": True, "required_btrfs_subvolumes": True, "efi_private_mask": True, "no_swap_partition_or_fstab_entry": True},
            "limitations": ["Offline read-only bootstrap layout/account audit; not running product services, production configuration or final OS acceptance."]}


def main(argv=None):
    try:
        values = sys.argv[1:] if argv is None else argv
        require(len(values) == 3, "registered-audit-arguments")
        report = audit(*values)
        status = 0
    except Denied as error:
        report, status = {"schema_version": 1, "error": "LAYOUT_AUDIT_DENIED", "check": str(error)}, 4
        if error.evidence is not None:
            report["evidence"] = error.evidence
    except (OSError, ValueError, subprocess.TimeoutExpired):
        report, status = {"schema_version": 1, "error": "LAYOUT_AUDIT_FAILED"}, 4
    print("AIOS_LAYOUT_REPORT=" + json.dumps(report, sort_keys=True))
    return status


if __name__ == "__main__":
    raise SystemExit(main())
