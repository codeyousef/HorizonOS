"""Locked package capabilities and a disposable Unix-only PostgreSQL fixture.

Runs only inside the enrolled guest job. Does not install declarations, touch
existing clusters, or claim graphical workflows or system-service activation.
"""
import configparser
import hashlib
import json
import os
from pathlib import Path
import pwd
import shlex
import shutil
import tempfile


def qualify(reference, locked, pure, catalog, run, scratch):
    expected = {"blender", "kate", "kcalc", "postgresql-17"}
    entries = {entry["id"]: entry for entry in catalog["content"]["packages"]}
    if set(entries) != expected or os.getuid() == 0:
        raise RuntimeError("catalog qualification needs reviewed packages and a nonroot fixture owner")
    outputs = json.loads(run(["nix", "build", "--json", "--no-link", *locked, *pure,
        "--option", "substituters", "https://cache.nixos.org",
        *[reference + "#lib.catalogPackages." + name for name in sorted(expected)]], timeout=1200))
    realized = {}
    for name in sorted(expected):
        path = Path(run(["nix", "eval", "--raw", *locked, *pure,
            reference + "#lib.catalogPackages." + name + ".outPath"]).decode())
        if str(path) not in [item["outputs"]["out"] for item in outputs]:
            raise RuntimeError("realized package differs from reviewed mapping")
        realized[name] = path
    applications = []
    with tempfile.TemporaryDirectory(prefix="catalog-app-", dir=scratch) as directory:
        home = Path(directory)
        runtime = home / "runtime"
        runtime.mkdir(mode=0o700)
        app_env = {"HOME": str(home), "XDG_CONFIG_HOME": str(home / "config"),
            "XDG_CACHE_HOME": str(home / "cache"), "XDG_DATA_HOME": str(home / "data"),
            "XDG_RUNTIME_DIR": str(runtime), "QT_QPA_PLATFORM": "offscreen"}
        for name in ("blender", "kate", "kcalc"):
            entry, package = entries[name], realized[name]
            binary = package / "bin" / name
            if not binary.is_file() or not os.access(binary, os.X_OK):
                raise RuntimeError("reviewed application executable is missing")
            version = run([str(binary), "--version"], timeout=30, extra_env=app_env).decode()
            if entry["version"] not in version:
                raise RuntimeError("actual application version differs from locked catalog")
            desktops = []
            for desktop_id in entry["desktop_ids"]:
                desktop = package / "share/applications" / desktop_id
                data = desktop.read_bytes()
                parser = configparser.ConfigParser(interpolation=None, strict=True)
                parser.read_string(data.decode())
                record = parser["Desktop Entry"]
                executable = shlex.split(record["Exec"])[0]
                if record["Type"] != "Application" or Path(executable).name not in entry["binaries"]:
                    raise RuntimeError("desktop launch capability differs from reviewed executable")
                if "/" in executable and Path(executable).resolve() != binary.resolve():
                    raise RuntimeError("desktop entry redirects outside its reviewed executable")
                desktops.append({"id": desktop_id, "sha256": hashlib.sha256(data).hexdigest(),
                    "exec": record["Exec"], "type": record["Type"]})
            applications.append({"id": name, "package": str(package), "version_output": version,
                "executable_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                "desktop_entries": desktops, "executable_and_desktop_capability_verified": True,
                "graphical_workflow_verified": False})
    postgres = realized["postgresql-17"]
    bin_path = postgres / "bin"
    version = run([str(bin_path / "postgres"), "--version"]).decode()
    if entries["postgresql-17"]["version"] not in version:
        raise RuntimeError("actual PostgreSQL major/version differs from catalog")
    # Retain the private cluster on a failed stop instead of deleting live data.
    with tempfile.TemporaryDirectory(prefix="catalog-pg-", dir=scratch, delete=False) as directory:
        root = Path(directory)
        data, socket = root / "data", root / "socket"
        socket.mkdir(mode=0o700)
        owner = pwd.getpwuid(os.getuid()).pw_name
        fixed_probe = [str(bin_path / "pg_isready"), "-h", str(socket), "-p", "65431",
            "-U", owner, "-d", "postgres", "-t", "2"]
        run(fixed_probe, expected=2, timeout=5)
        run([str(bin_path / "initdb"), "-D", str(data), "--no-locale", "--encoding=UTF8",
            "--auth-local=peer", "--auth-host=reject"], timeout=30)
        socket_value = str(socket).replace("'", "''")
        with (data / "postgresql.conf").open("a") as configuration:
            configuration.write("\nlisten_addresses = ''\nport = 65431\nunix_socket_directories = '" + socket_value + "'\n")
        ctl = [str(bin_path / "pg_ctl"), "-D", str(data), "-w", "-t", "20"]
        try:
            run([*ctl, "-l", str(root / "server.log"), "start"], timeout=30)
            run(fixed_probe, timeout=5)
            psql = [str(bin_path / "psql"), "--no-psqlrc", "-h", str(socket), "-p", "65431",
                "-U", owner, "-d", "postgres", "-At", "-c"]
            settings = run([*psql, "SELECT current_user, current_setting('server_version'), current_setting('listen_addresses'), current_setting('unix_socket_directories');"], timeout=5).decode().strip().split("|")
            if settings != [owner, entries["postgresql-17"]["version"], "", str(socket)]:
                raise RuntimeError("fixture listener/user/version differs from fixed Unix-only plan")
            pid = int((data / "postmaster.pid").read_text().splitlines()[0])
            if Path(f"/proc/{pid}").stat().st_uid != os.getuid():
                raise RuntimeError("fixture PostgreSQL runs under another identity")
            inodes = set()
            for descriptor in Path(f"/proc/{pid}/fd").iterdir():
                try:
                    target = os.readlink(descriptor)
                except FileNotFoundError:
                    continue
                if target.startswith("socket:["):
                    inodes.add(target[8:-1])
            for protocol in ("tcp", "tcp6"):
                for row in Path("/proc/net/" + protocol).read_text().splitlines()[1:]:
                    if row.split()[9] in inodes:
                        raise RuntimeError("Unix-only fixture unexpectedly owns a TCP socket")
            run([*psql, "CREATE TABLE horizon_catalog_fixture (value integer); INSERT INTO horizon_catalog_fixture VALUES (17);"], timeout=5)
            if run([*psql, "SELECT value FROM horizon_catalog_fixture;"], timeout=5).strip() != b"17":
                raise RuntimeError("PostgreSQL fixture failed actual query capability")
            result = {"package": str(postgres), "version_output": version, "fixture_uid": os.getuid(),
                "fixture_user": owner, "probe": fixed_probe, "readiness_verified": True,
                "unix_only_listener_verified": True, "query_verified": True,
                "existing_system_data_touched": False, "system_service_activation_verified": False}
        finally:
            # Never remove a cluster while its server may still be running.
            if (data / "postmaster.pid").exists():
                run([*ctl, "-m", "fast", "stop"], timeout=30)
            if (data / "postmaster.pid").exists():
                raise RuntimeError("fixture server did not stop; cluster cleanup refused")
        run(fixed_probe, expected=2, timeout=5)
        result["teardown_verified"] = True
        shutil.rmtree(root)
    return {"applications": applications, "postgresql_fixture": result,
        "evidence_kind": "real-locked-package-capabilities-with-disposable-nonroot-database",
        "package_outputs": outputs}
