#!/usr/bin/env python3
"""Disposable tester qualification of exact canonical native provider/broker.

No approval bypass or general privileged command runner. Temporary runtime
units never replace an active service or existing user override; exact owned
bytes are removed on cleanup, restoring the prior inactive package fragment.
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
                      "ProtectProc","MemoryMax","TasksMax","RuntimeDirectoryMode","PartOf","Requisite","InvocationID"]
            result = ctl("show",unit,*[v for key in fields for v in ("--property",key)]).stdout.decode()
            return dict(line.split("=",1) for line in result.splitlines() if "=" in line)
        if show("graphical-session.target")["ActiveState"] != "active":
            raise RuntimeError("actual graphical session target is inactive")
        owner = subprocess.run(["busctl","--user","call","org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus",
                                "NameHasOwner","s","org.aios.Session1"],env=env,check=True,stdout=subprocess.PIPE,timeout=5)
        if owner.stdout.strip()!=b"b false":
            raise RuntimeError("existing public broker must not be replaced")
        units={name:(core/"share/systemd/user"/name).read_bytes() for name in ("aios-sessiond.service","aios-ui-agent.service")}
        before={name:show(name) for name in units}
        if any(v["ActiveState"] not in ("inactive","failed") or v["MainPID"]!="0" for v in before.values()):
            raise RuntimeError("canonical service is already active")
        directory=runtime/"systemd/user"
        directory.mkdir(mode=0o700,parents=True,exist_ok=True)
        if directory.resolve()!=directory or directory.stat().st_uid!=1001:
            raise RuntimeError("unsafe runtime unit directory")
        links={name:directory/name for name in units}
        if any(path.exists() or path.is_symlink() for path in links.values()):
            raise RuntimeError("existing user override must not be replaced")
        if (runtime/"aios/session.sock").exists() or (runtime/"aios-ui/provider.sock").exists():
            raise RuntimeError("existing native endpoint must not be replaced")
        created=[]
        def owns(name):
            path=links[name];meta=path.lstat()
            return stat.S_ISREG(meta.st_mode) and meta.st_uid==1001 and meta.st_nlink==1 and stat.S_IMODE(meta.st_mode)==0o600 and path.read_bytes()==units[name]
        try:
            for name,data in units.items():
                if len(data)>65536:raise RuntimeError("oversized unit")
                fd=os.open(links[name],os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
                with os.fdopen(fd,"wb") as output:output.write(data)
                created.append(name)
            ctl("daemon-reload")
            for name in units:ctl("start",name)
            effective={name:show(name) for name in units}
            for name,props in effective.items():
                if props["ActiveState"]!="active" or int(props["MainPID"])<=1 or props["FragmentPath"]!=str(links[name]) or not owns(name):
                    raise RuntimeError("exact managed service did not start: "+name)
                required={"NoNewPrivileges":"yes","MemoryMax":"268435456","TasksMax":"64","RuntimeDirectoryMode":"0700"}
                if name=="aios-sessiond.service":
                    required.update({"PrivateNetwork":"yes","PrivateDevices":"yes","ProtectHome":"tmpfs","ProtectSystem":"strict","ProtectProc":"invisible"})
                else:
                    required.update({"PrivateNetwork":"no","PrivateDevices":"no","ProtectHome":"no","ProtectSystem":"no","ProtectProc":"default","PrivateTmp":"no"})
                for key,value in required.items():
                    if props[key]!=value:raise RuntimeError("actual hardening mismatch: "+name+" "+key)
            if effective["aios-sessiond.service"]["PrivateUsers"] not in ("yes","self") or effective["aios-ui-agent.service"]["PrivateUsers"]!="no":
                raise RuntimeError("namespace plan mismatch")
            if "graphical-session.target" not in effective["aios-ui-agent.service"]["PartOf"].split() or "graphical-session.target" not in effective["aios-ui-agent.service"]["Requisite"].split():
                raise RuntimeError("provider graphical lifetime mismatch")
            native_status=Path('/proc/'+effective['aios-ui-agent.service']['MainPID']+'/status').read_text()
            native_fields=dict(line.split(':',1) for line in native_status.splitlines() if ':' in line)
            if any(int(native_fields[key].strip(),16)!=0 for key in ('CapEff','CapPrm','CapInh','CapAmb')) or native_fields['NoNewPrivs'].strip()!='1':
                raise RuntimeError('native provider capabilities or privilege restriction mismatch')
            endpoint=runtime/"aios-ui/provider.sock"
            deadline=time.monotonic()+5
            while not endpoint.exists():
                if time.monotonic()>=deadline:raise RuntimeError("provider endpoint missing")
                time.sleep(0.01)
            meta=endpoint.lstat()
            if not stat.S_ISSOCK(meta.st_mode) or meta.st_uid!=1001 or stat.S_IMODE(meta.st_mode)!=0o600:
                raise RuntimeError("provider socket mode mismatch")
            with socket.socket(socket.AF_UNIX) as forged:
                forged.settimeout(3);forged.connect(str(endpoint))
                forged.sendall(b'\xa7forged approved=true')
                try:reply=forged.recv(4096)
                except ConnectionResetError:reply=b""
                if reply:raise RuntimeError("unmanaged client reached provider authority")
            scenario={**env,"AIOS_NATIVE_BRIDGE_SCENARIO":"disposable-provider-v1"}
            result=subprocess.run(["nix","develop","--no-update-lock-file","--no-write-lock-file","path:"+str(release),"--command",
                                   "cargo","test","--locked","-p","aios-session","--test","accessibility_native","--","--ignored","--nocapture"],
                                  env=scenario,check=False,stdout=subprocess.PIPE,timeout=900)
            output=result.stdout.decode()
            print(output,flush=True)
            if result.returncode:raise RuntimeError('native bridge qualification failed')
            if "NATIVE_PROVIDER_BRIDGE=" not in output:raise RuntimeError("native bridge scenario did not run")
            print("NATIVE_MANAGED_UI_PROVIDER="+json.dumps({"evidence_kind":"actual-exact-managed-package-original-fd-bridge-and-owned-native-assistive-input-fixture-not-human-approval",
                "uid":1001,"outputs":[str(p) for p in paths],"before":before,"effective":effective,
                "unit_sha256":{name:hashlib.sha256(data).hexdigest() for name,data in units.items()},"unmanaged_client_denied":True,
                "native_privilege_state":{key:native_fields[key].strip() for key in ('CapEff','CapPrm','CapInh','CapAmb','NoNewPrivs')},
                "provider_binary_sha256":hashlib.sha256(provider.read_bytes()).hexdigest(),"exact_package_unit_bytes":True}),flush=True)
        except Exception:
            print("NATIVE_PROVIDER_FAILURE="+json.dumps({name:show(name) for name in units}),flush=True)
            for name in units:
                journal=subprocess.run(["journalctl","--user","--user-unit="+name,"--boot","--lines=15","--no-pager","--output=json",
                                        "--output-fields=MESSAGE,PRIORITY,_BOOT_ID,_UID"],env=env,stdout=subprocess.PIPE,timeout=5)
                print("NATIVE_OWN_UNIT_JOURNAL="+journal.stdout.decode(errors="replace"),flush=True)
            raise
        finally:
            for name in reversed(created):
                if not owns(name):raise RuntimeError("runtime service ownership changed; cleanup refused")
                props=show(name)
                if props["FragmentPath"]!=str(links[name]):raise RuntimeError("service fragment changed; cleanup refused")
                ctl("stop",name);ctl("reset-failed",name,check=False)
                if show(name)["MainPID"]!="0":raise RuntimeError("owned service did not stop")
                links[name].unlink()
            ctl("daemon-reload")


if __name__=="__main__":main()
