#!/usr/bin/env python3
"""Observe fixed initial-root preflight evidence on an enrolled running image."""
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import snapshot


def installed_diagnostics():
    """Read only fixed public installed authority and its declared template."""
    records = {}
    root = Path("/run/current-system/etc/aios")
    for name in ("target-authority.json", "template-authority.json", "approval-authority.json"):
        path = root / name
        data = path.read_bytes()
        if len(data) > 65536:
            raise ValueError("installed record exceeds observation limit")
        value = snapshot.decode(data)
        records[name] = {"value": value, "sha256": hashlib.sha256(data).hexdigest(),
            "canonical": snapshot.canonical(value) == data, "resolved": str(path.resolve(strict=True))}
    authority = records["template-authority.json"]["value"]
    template = Path(authority["template_path"])
    if template.parent != Path("/nix/store") or template.is_symlink():
        raise ValueError("installed template is not a direct store object")
    data = (template / "template.json").read_bytes()
    if len(data) > 1024 * 1024:
        raise ValueError("template manifest exceeds observation limit")
    manifest = snapshot.decode(data)
    if len(manifest["files"]) > 4096:
        raise ValueError("template entry count exceeds observation limit")
    mismatches = []
    for entry in manifest["files"]:
        snapshot.relative_path(entry["path"])
        path = template / entry["path"]
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_size > 16 * 1024 * 1024:
            raise ValueError("unsafe installed template observation")
        content = path.read_bytes()
        if len(content) != entry["size"] or hashlib.sha256(content).hexdigest() != entry["sha256"]:
            mismatches.append(entry["path"])
    observed = {str(path.relative_to(template)) for path in template.rglob("*") if not path.is_dir()}
    wanted = {entry["path"] for entry in manifest["files"]} | {"template.json"}
    policy = records["approval-authority.json"]["value"]
    package = Path(policy["polkit_package"])
    if package.parent != Path("/nix/store") or package.is_symlink():
        raise ValueError("polkit package is not a direct store object")
    policy_inventory = []
    for directory in (Path("/etc/polkit-1/rules.d"),
                      Path("/run/current-system/sw/share/polkit-1/rules.d"),
                      Path("/run/current-system/sw/share/polkit-1/actions"),
                      package / "share/polkit-1/rules.d", package / "share/polkit-1/actions"):
        files = []
        if directory.exists():
            entries = list(directory.iterdir())
            if len(entries) > 4096:
                raise ValueError("policy inventory observation limit")
            for path in sorted(entries):
                info = path.stat()
                files.append({"name":path.name,"size":info.st_size,"regular":stat.S_ISREG(info.st_mode),
                    "uid":info.st_uid,"mode":stat.S_IMODE(info.st_mode)})
        policy_inventory.append({"path":str(directory),"exists":directory.exists(),"files":files})
    return {"evidence_kind":"read-only-installed-public-records", "records":records,
        "policy_inventory":policy_inventory,
        "template_manifest_sha256":hashlib.sha256(data).hexdigest(),
        "template_manifest_matches_authority":hashlib.sha256(data).hexdigest() == authority["manifest_sha256"],
        "template_manifest_canonical":snapshot.canonical(manifest) == data,
        "template_content_mismatches":mismatches,
        "template_extra_files":sorted(observed-wanted),"template_missing_files":sorted(wanted-observed)}


def protected_preflight(*, guard=False):
    for name in ("/", "/run", "/run/aios-initial-preflight"):
        info = Path(name).lstat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022:
            raise ValueError("unprotected initial preflight directory")
    name = "guard.json" if guard else "result.json"
    descriptor = os.open("/run/aios-initial-preflight/" + name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as handle:
        before = os.fstat(handle.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_uid != 0 or stat.S_IMODE(before.st_mode) != 0o644 or before.st_size > 16384:
            raise ValueError("unprotected initial preflight result")
        data = handle.read(16385)
        after = os.fstat(handle.fileno())
        if len(data) != before.st_size or (before.st_ino,before.st_size,before.st_mtime_ns,before.st_ctime_ns) != (after.st_ino,after.st_size,after.st_mtime_ns,after.st_ctime_ns):
            raise ValueError("initial preflight changed during capture")
    return snapshot.decode(data), hashlib.sha256(data).hexdigest()


def main():
    identity = snapshot.identity()
    commands = []
    services = {}
    for unit in ("aios-initial-preflight.service", "polkit.service", "sshd.service"):
        if snapshot.identity() != identity:
            raise ValueError("runtime target changed")
        argv = ["systemctl","show","--no-pager","--property=LoadState,ActiveState,SubState,Result,ExecMainStatus,MainPID,User,FragmentPath",unit]
        result = subprocess.run(argv, capture_output=True, timeout=10, check=False)
        commands.append({"argv":argv,"upstream_exit":result.returncode})
        if result.returncode:
            raise ValueError("fixed runtime service observation failed")
        services[unit] = dict(line.split("=",1) for line in result.stdout.decode().splitlines() if "=" in line)
    report = {"schema_version":1,"evidence_kind":"actual-installed-root-preflight-observation",
        "target_identity":identity,"services":services,"commands":commands,
        "native_authorization_verified":False,"trusted_confirmation_verified":False,
        "root_developer_registration_verified":False,"guarded_activation_verified":False}
    try:
        report["installed_diagnostics"] = installed_diagnostics()
        proof, digest = protected_preflight()
        if proof["uid"] != 0 or proof["effective_uid"] != 0 or proof["boot_id"] != identity["boot_id"] or proof["installation_uuid"] != identity["installation_uuid"]:
            raise ValueError("initial root preflight identity differs")
        if any(proof[key] for key in ("authenticated_caller_verified","native_authorization_verified","trusted_confirmation_verified","product_effects_performed")):
            raise ValueError("initial preflight claimed unsupported authority/effects")
        executable = Path("/run/current-system/sw/bin/aios-execd").resolve(strict=True)
        if proof["executable"] != str(executable) or proof["executable_sha256"] != hashlib.sha256(executable.read_bytes()).hexdigest():
            raise ValueError("initial preflight executable differs")
        report.update(root_preflight=proof,root_preflight_sha256=digest)
    except (OSError, ValueError, KeyError) as error:
        report["failure"] = type(error).__name__
        print("AIOS_INSTALLED_RUNTIME " + json.dumps(report,sort_keys=True),flush=True)
        raise
    if snapshot.identity() != identity:
        raise ValueError("runtime target changed after observation")
    print("AIOS_INSTALLED_RUNTIME " + json.dumps(report,sort_keys=True),flush=True)
    if not proof["native_preflight_verified"] or services["aios-initial-preflight.service"]["Result"] != "success":
        raise RuntimeError("installed native root preflight has not passed")


if __name__ == "__main__":
    main()
