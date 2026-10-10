#!/usr/bin/env python3
"""Disposable tester qualification of exact canonical native provider/broker.

No approval bypass or general privileged command runner. Installed services must match the exact built package and canonical fragment.
Temporary runtime units never replace an active service or existing user
override; exact owned bytes are removed on cleanup.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import socket
import stat
import subprocess
import time
from service_inspection_smoke import products


def main():
    if Path("/etc/aios/desktop-test-profile").read_text().strip() != "synthetic-disposable-plasma-wayland-v1" or os.geteuid() != 1001:
        raise RuntimeError("requires the registered disposable native tester")
    release = Path(__file__).resolve().parents[2]
    paths, binaries = products()
    core = binaries["aios-sessiond"].parents[1]
    provider = core / "bin/aios-ui-agent"
    if not provider.is_file():
        raise RuntimeError("native provider absent from the actual core package")
    runtime = Path("/run/user/1001")
    meta = runtime.lstat()
    if runtime.resolve() != runtime or not stat.S_ISDIR(meta.st_mode) or meta.st_uid != 1001 or meta.st_mode & 0o077:
        raise RuntimeError("unsafe tester runtime")
    env = {**os.environ, "XDG_RUNTIME_DIR": str(runtime), "DBUS_SESSION_BUS_ADDRESS": "unix:path=" + str(runtime / "bus")}
    qualification = runtime / "aios-qualification"
    qualification.mkdir(mode=0o700, exist_ok=True)
    meta = qualification.lstat()
    if not stat.S_ISDIR(meta.st_mode) or qualification.resolve() != qualification or meta.st_uid != 1001 or meta.st_mode & 0o077:
        raise RuntimeError("unsafe qualification directory")
    fd = os.open(qualification / "public-session.lock", os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd,"r+") as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        def ctl(*args,check=True):
            return subprocess.run(["systemctl","--user",*args],env=env,check=check,stdout=subprocess.PIPE,timeout=20)
        def show(unit):
            fields = ["LoadState","ActiveState","SubState","MainPID","FragmentPath","Type","Result","ExecMainStatus",
                      "NoNewPrivileges","PrivateNetwork","PrivateUsers","PrivateDevices","PrivateTmp","ProtectHome","ProtectSystem",
                      "ProtectProc","MemoryMax","TasksMax","RuntimeDirectoryMode","PartOf","Requisite","InvocationID","ExecStart"]
            result = ctl("show",unit,*[v for key in fields for v in ("--property",key)]).stdout.decode()
            return dict(line.split("=",1) for line in result.splitlines() if "=" in line)
        if show("graphical-session.target")["ActiveState"] != "active":
            raise RuntimeError("actual graphical session target is inactive")
        owner = subprocess.run(["busctl","--user","call","org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus",
                                "NameHasOwner","s","org.aios.Session1"],env=env,check=True,stdout=subprocess.PIPE,timeout=5)
        if owner.stdout.strip() not in (b"b false",b"b true"):
            raise RuntimeError("malformed public broker ownership")
        installed=owner.stdout.strip()==b"b true"
        units={name:(core/"share/systemd/user"/name).read_bytes() for name in ("aios-sessiond.service","aios-ui-agent.service","aios-processd.service")}
        before={name:show(name) for name in units}
        def canonical(name,props):
            fragment=Path(props["FragmentPath"])
            if fragment.resolve()!=core/"share/systemd/user"/name or fragment.read_bytes()!=units[name]:
                raise RuntimeError("installed canonical fragment differs from exact package: "+name)
            if props["ActiveState"]=="active":
                pid=int(props["MainPID"])
                if pid<=1 or Path(f"/proc/{pid}/exe").resolve()!=core/"bin"/name.removesuffix(".service"):
                    raise RuntimeError("installed live executable differs from exact package: "+name)
                if Path(f"/proc/{pid}/cmdline").read_bytes()!=os.fsencode(core/"bin"/name.removesuffix(".service"))+b"\0":
                    raise RuntimeError("installed native argv differs from fixed unit: "+name)
            elif props["ActiveState"] not in ("inactive","failed") or props["MainPID"]!="0":
                raise RuntimeError("installed unit is in an unsafe state: "+name)
        if installed:
            for name,props in before.items():canonical(name,props)
            if before["aios-sessiond.service"]["ActiveState"]!="active":
                raise RuntimeError("public owner is not the exact installed broker")
        elif any(v["ActiveState"] not in ("inactive","failed") or v["MainPID"]!="0" for v in before.values()):
            raise RuntimeError("canonical service is already active")
        directory=runtime/"systemd/user"
        if not installed:directory.mkdir(mode=0o700,parents=True,exist_ok=True)
        if not installed and (directory.resolve()!=directory or directory.stat().st_uid!=1001):
            raise RuntimeError("unsafe runtime unit directory")
        links={name:directory/name for name in units}
        if any(path.exists() or path.is_symlink() for path in links.values()):
            raise RuntimeError("existing user override must not be replaced")
        if not installed and any((runtime/name).exists() for name in ("aios/session.sock","aios-ui/provider.sock","aios-process/provider.sock")):
            raise RuntimeError("existing native endpoint must not be replaced")
        created=[];started=[];effective={}
        def owns(name):
            path=links[name];meta=path.lstat()
            return stat.S_ISREG(meta.st_mode) and meta.st_uid==1001 and meta.st_nlink==1 and stat.S_IMODE(meta.st_mode)==0o600 and path.read_bytes()==units[name]
        try:
            for name,data in (() if installed else units.items()):
                if len(data)>65536:raise RuntimeError("oversized unit")
                fd=os.open(links[name],os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
                with os.fdopen(fd,"wb") as output:output.write(data)
                created.append(name)
            if not installed:ctl("daemon-reload")
            for name in units:
                if before[name]["ActiveState"]!="active":
                    if installed:canonical(name,show(name))
                    ctl("start",name);started.append(name)
            effective={name:show(name) for name in units}
            for name,props in effective.items():
                if installed:canonical(name,props)
                elif props["FragmentPath"]!=str(links[name]) or not owns(name):
                    raise RuntimeError("exact runtime fragment changed: "+name)
                if props["ActiveState"]!="active" or int(props["MainPID"])<=1:
                    raise RuntimeError("exact managed service did not start: "+name)
                required={"NoNewPrivileges":"yes","MemoryMax":"268435456","TasksMax":"64","RuntimeDirectoryMode":"0700"}
                if name=="aios-sessiond.service":
                    required.update({"PrivateNetwork":"yes","PrivateDevices":"yes","ProtectHome":"no","ProtectSystem":"strict","ProtectProc":"invisible"})
                else:
                    required.update({"PrivateNetwork":"no","PrivateDevices":"no","ProtectHome":"no","ProtectSystem":"no","ProtectProc":"default","PrivateTmp":"no"})
                for key,value in required.items():
                    if props[key]!=value:raise RuntimeError("actual hardening mismatch: "+name+" "+key)
            if effective["aios-sessiond.service"]["PrivateUsers"]!="no" or any(effective[name]["PrivateUsers"]!="no" for name in ("aios-ui-agent.service","aios-processd.service")):
                raise RuntimeError("namespace plan mismatch")
            if "graphical-session.target" not in effective["aios-ui-agent.service"]["PartOf"].split() or "graphical-session.target" not in effective["aios-ui-agent.service"]["Requisite"].split():
                raise RuntimeError("provider graphical lifetime mismatch")
            native_privileges={}
            for name,endpoint in (("aios-ui-agent.service",runtime/"aios-ui/provider.sock"),("aios-processd.service",runtime/"aios-process/provider.sock")):
                native_status=Path('/proc/'+effective[name]['MainPID']+'/status').read_text()
                native_fields=dict(line.split(':',1) for line in native_status.splitlines() if ':' in line)
                if any(int(native_fields[key].strip(),16)!=0 for key in ('CapEff','CapPrm','CapInh','CapAmb')) or native_fields['NoNewPrivs'].strip()!='1':
                    raise RuntimeError('native provider capabilities or privilege restriction mismatch: '+name)
                native_privileges[name]={key:native_fields[key].strip() for key in ('CapEff','CapPrm','CapInh','CapAmb','NoNewPrivs')}
                deadline=time.monotonic()+5
                while not endpoint.exists():
                    if time.monotonic()>=deadline:raise RuntimeError("provider endpoint missing: "+name)
                    time.sleep(0.01)
                meta=endpoint.lstat()
                if not stat.S_ISSOCK(meta.st_mode) or meta.st_uid!=1001 or stat.S_IMODE(meta.st_mode)!=0o600:
                    raise RuntimeError("provider socket mode mismatch: "+name)
                with socket.socket(socket.AF_UNIX) as forged:
                    forged.settimeout(3);forged.connect(str(endpoint))
                    forged.sendall(b'\xa7forged approved=true')
                    try:reply=forged.recv(4096)
                    except ConnectionResetError:reply=b""
                    if reply:raise RuntimeError("unmanaged client reached provider authority: "+name)
            scenario={**env,"AIOS_NATIVE_BRIDGE_SCENARIO":"disposable-provider-v1"}
            result=subprocess.run(["nix","develop","--no-update-lock-file","--no-write-lock-file","path:"+str(release),"--command",
                                   "cargo","test","--locked","-p","aios-session","--test","accessibility_native","--","--ignored","--nocapture"],
                                  env=scenario,check=False,stdout=subprocess.PIPE,timeout=900)
            output=result.stdout.decode()
            print(output,flush=True)
            if result.returncode:raise RuntimeError('native bridge qualification failed')
            if "NATIVE_PROVIDER_BRIDGE=" not in output:raise RuntimeError("native bridge scenario did not run")
            if "NATIVE_BUS_WINDOW_DISCOVERY=" not in output:raise RuntimeError("public native D-Bus discovery scenario did not run")
            process=subprocess.run(["nix","develop","--no-update-lock-file","--no-write-lock-file","path:"+str(release),"--command",
                                    "cargo","test","--locked","-p","aios-session","--test","process_termination_native","--","--ignored","--nocapture"],
                                   env=scenario,check=False,stdout=subprocess.PIPE,timeout=900)
            process_output=process.stdout.decode();print(process_output,flush=True)
            if process.returncode or "NATIVE_MANAGED_PROCESS_TERMINATION=" not in process_output:
                raise RuntimeError("native managed termination qualification failed")
            refusals=subprocess.run(["nix","develop","--no-update-lock-file","--no-write-lock-file","path:"+str(release),"--command",
                                     "cargo","test","--locked","--manifest-path",str(release/"tests/native-runtime/Cargo.toml"),
                                     "--test","process_refusals","--","--ignored","--nocapture"],
                                    env=scenario,check=False,stdout=subprocess.PIPE,timeout=900)
            refusal_output=refusals.stdout.decode();print(refusal_output,flush=True)
            if refusals.returncode or "NATIVE_MANAGED_PROCESS_REFUSALS=" not in refusal_output:
                raise RuntimeError("native managed termination negative qualification failed")
            origin=subprocess.run(["nix","develop","--no-update-lock-file","--no-write-lock-file","path:"+str(release),"--command",
                                   "cargo","test","--locked","-p","aios-session","--lib","user_bus::tests::","--","--ignored","--nocapture"],
                                  env=scenario,check=False,stdout=subprocess.PIPE,timeout=900)
            native_origin=origin.stdout.decode();print(native_origin,flush=True)
            if origin.returncode or "NATIVE_BUS_ORIGIN=" not in native_origin:raise RuntimeError("native D-Bus origin qualification failed")
            print("NATIVE_MANAGED_UI_PROVIDER="+json.dumps({"evidence_kind":"actual-exact-managed-package-original-fd-bridge-and-owned-native-assistive-input-fixture-not-human-approval",
                "uid":1001,"installed_exact_package":installed,"outputs":[str(p) for p in paths],"before":before,"effective":effective,
                "unit_sha256":{name:hashlib.sha256(data).hexdigest() for name,data in units.items()},"unmanaged_client_denied":True,
                "native_privilege_state":native_privileges,
                "provider_binary_sha256":hashlib.sha256(provider.read_bytes()).hexdigest(),"exact_package_unit_bytes":True}),flush=True)
        except Exception:
            print("NATIVE_PROVIDER_FAILURE="+json.dumps({name:show(name) for name in units}),flush=True)
            for name in units:
                journal=subprocess.run(["journalctl","--user","--user-unit="+name,"--boot","--lines=15","--no-pager","--output=json",
                                        "--output-fields=MESSAGE,PRIORITY,_BOOT_ID,_UID"],env=env,stdout=subprocess.PIPE,timeout=5)
                print("NATIVE_OWN_UNIT_JOURNAL="+journal.stdout.decode(errors="replace"),flush=True)
            raise
        finally:
            if installed:
                for name in reversed(started):
                    props=show(name);canonical(name,props)
                    if name in effective and any(props[key]!=effective[name][key] for key in ("MainPID","InvocationID","ExecStart")):
                        raise RuntimeError("started canonical unit identity changed; cleanup refused")
                    ctl("stop",name)
                    if show(name)["MainPID"]!="0":raise RuntimeError("started canonical unit did not stop")
            for name in reversed(created):
                if not owns(name):raise RuntimeError("runtime service ownership changed; cleanup refused")
                props=show(name)
                if props["FragmentPath"]!=str(links[name]):raise RuntimeError("service fragment changed; cleanup refused")
                ctl("stop",name);ctl("reset-failed",name,check=False)
                if show(name)["MainPID"]!="0":raise RuntimeError("owned service did not stop")
                links[name].unlink()
            if created:ctl("daemon-reload")


if __name__=="__main__":main()
