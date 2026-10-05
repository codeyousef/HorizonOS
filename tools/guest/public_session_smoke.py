#!/usr/bin/env python3
"""Exercise the real packaged user unit; never replace an existing service."""
import hashlib
import uuid
import json
import fcntl
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import time
from service_inspection_smoke import products
from model_public_fixture import require_system_answer

UNIT = "aios-session-acceptance-" + uuid.uuid4().hex + ".service"


def installed_products():
    binaries={}; paths=[]
    for program,package in (("aios-sessiond","aios-core"),("aiosctl","aios-cli")):
        binary=Path("/run/current-system/sw/bin",program).resolve(strict=True)
        info=binary.stat(); output=binary.parents[1]
        if (not re.fullmatch(r"/nix/store/[a-z0-9]{32}-"+package+r"-[A-Za-z0-9._+-]+",str(output))
                or binary!=output/"bin"/program or not stat.S_ISREG(info.st_mode)
                or info.st_uid!=0 or info.st_mode&0o222 or not info.st_mode&0o111):
            raise RuntimeError("installed client/broker is not a protected store executable")
        binaries[program]=binary;paths.append(output)
    return paths,binaries


def main():
    installed_model = sys.argv[1:] == ["--installed-model"]
    if sys.argv[1:] and not installed_model:
        raise RuntimeError("unregistered user-service scenario")
    if installed_model and (os.geteuid() == 0 or Path("/etc/aios/model-test-profile").read_text().strip() != "installed-normal-cpu-model-v1"):
        raise RuntimeError("installed inference requires the model test image and normal UID")
    paths, binaries = installed_products() if installed_model else products()
    runtime = Path(f"/run/user/{os.geteuid()}")
    info = runtime.lstat()
    if runtime.resolve() != runtime or not stat.S_ISDIR(info.st_mode) or info.st_uid != os.geteuid() or info.st_mode & 0o077:
        raise RuntimeError("private user runtime identity mismatch")
    env = {**os.environ, "XDG_RUNTIME_DIR": str(runtime), "DBUS_SESSION_BUS_ADDRESS": "unix:path=" + str(runtime / "bus")}
    qualification = runtime / "aios-qualification"
    qualification.mkdir(mode=0o700, exist_ok=True)
    info = qualification.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.geteuid() or info.st_mode & 0o077:
        raise RuntimeError("unsafe qualification directory")
    descriptor = os.open(qualification / "public-session.lock", os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    lock = os.fdopen(descriptor, "r+")
    fcntl.flock(lock, fcntl.LOCK_EX)
    def ctl(*arguments, check=True):
        return subprocess.run(["systemctl", "--user", *arguments], env=env, check=check, stdout=subprocess.PIPE, timeout=20)
    def show():
        selected = ["LoadState", "ActiveState", "SubState", "Result", "ExecMainCode", "ExecMainStatus", "MainPID", "FragmentPath", "NoNewPrivileges", "PrivateNetwork", "ProtectHome", "ProtectSystem", "MemoryMax", "TasksMax", "RuntimeDirectoryMode"]
        arguments = [part for key in selected for part in ("--property", key)]
        output = ctl("show", UNIT, *arguments).stdout.decode()
        return dict(line.split("=", 1) for line in output.splitlines() if "=" in line)
    if show()["LoadState"] != "not-found":
        raise RuntimeError("qualification unit must not replace an existing unit")
    has_owner = subprocess.run(["busctl", "--user", "call", "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "NameHasOwner", "s", "org.aios.Session1"], env=env, check=True, stdout=subprocess.PIPE, timeout=5)
    if has_owner.stdout.strip() != b"b false":
        raise RuntimeError("existing public broker owner must not be replaced")
    unit_file = binaries["aios-sessiond"].parents[1] / "share/systemd/user/aios-sessiond.service"
    unit_bytes = unit_file.read_bytes()
    if len(unit_bytes)>65536:
        raise RuntimeError("unit exceeds qualification size limit")
    runtime_link = runtime / "systemd/user" / UNIT
    if runtime_link.exists() or runtime_link.is_symlink():
        raise RuntimeError("existing runtime unit must not be replaced")
    runtime_link.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    if runtime_link.parent.resolve()!=runtime_link.parent or runtime_link.parent.stat().st_uid!=os.geteuid():
        raise RuntimeError("runtime unit directory ownership mismatch")
    descriptor=os.open(runtime_link,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
    with os.fdopen(descriptor,"wb") as handle:
        handle.write(unit_bytes)
    def owns_link():
        info=runtime_link.lstat()
        return stat.S_ISREG(info.st_mode) and info.st_uid==os.geteuid() and info.st_nlink==1 and stat.S_IMODE(info.st_mode)==0o600 and runtime_link.read_bytes()==unit_bytes
    def owns_fragment(properties):
        fragment = properties.get("FragmentPath", "")
        return bool(fragment) and Path(fragment)==runtime_link and owns_link()
    try:
        ctl("daemon-reload")
        ctl("start", UNIT)
        properties = show()
        if not owns_fragment(properties) or properties["ActiveState"] != "active" or int(properties["MainPID"]) <= 1:
            raise RuntimeError("packaged user service did not become active")
        for key, value in {"NoNewPrivileges":"yes", "PrivateNetwork":"yes", "ProtectHome":"tmpfs", "ProtectSystem":"strict", "MemoryMax":"268435456", "TasksMax":"64", "RuntimeDirectoryMode":"0700"}.items():
            if properties[key] != value:
                raise RuntimeError("effective unit hardening mismatch: " + key)
        socket = runtime / "aios/session.sock"
        deadline = time.monotonic() + 5
        while not socket.exists():
            if time.monotonic() >= deadline:
                raise RuntimeError("packaged private socket missing")
            time.sleep(0.01)
        if stat.S_IMODE(socket.stat().st_mode) != 0o600:
            raise RuntimeError("private socket mode mismatch")
        def cli(*arguments, timeout=10):
            return subprocess.run([str(binaries["aiosctl"]), *arguments], env=env, stdout=subprocess.PIPE, timeout=timeout)
        status = cli("status", "--json")
        if status.returncode != 0:
            raise RuntimeError("packaged status client failed")
        capabilities = json.loads(status.stdout)
        if capabilities["transport"] != "session-dbus" or capabilities["ui_enabled"] or capabilities["inference_available"]:
            raise RuntimeError("session availability scope mismatch")
        ui = cli("ui", "select-session", "aios-no-such-session", "--json")
        ui_denial = json.loads(ui.stdout)
        if ui.returncode != 1 or ui_denial["error"]["code"] != "TARGET_NOT_FOUND":
            raise RuntimeError("hardened UI observer did not return the stable missing-session error")
        question = cli("ask", "What operating system is running? Cite the provided observation.", "--json", timeout=100 if installed_model else 10)
        answer = json.loads(question.stdout)
        print("AIOS_USER_MODEL_ANSWER=" + json.dumps({"installed_model": installed_model, "upstream_exit": question.returncode, "answer": answer}), flush=True)
        service_answer=None
        if installed_model:
            require_system_answer({'upstream_exit':question.returncode,'answer':answer},Path('/proc/sys/kernel/random/boot_id').read_text().strip())
            if question.returncode != 0 or answer["state"] != "completed" or answer["error"] is not None or answer["mutation_performed"]:
                raise RuntimeError("hardened user broker did not reach the actual installed model")
            output = answer["output"]
            fresh_ids={i for observation in output["evidence"] for i in observation["evidence_ids"]}
            cited=set(output["response"].get("evidence_ids",[]))
            if (output["response"]["kind"] != "answer"
                    or not cited or not cited.issubset(fresh_ids)
                    or any(not observation["complete"] or observation["source"]["provider"]!="aios-system" or observation["data"]["os_id"]!="nixos" for observation in output["evidence"])
                    or not output["local_cpu"] or output["mutation_performed"]):
                raise RuntimeError("installed inference did not return independently enrolled system evidence")
            selected=cli("ask","Is the selected sshd service running? Inspect its native status and cite the resulting service evidence.","--json","--service","sshd.service",timeout=100)
            service_answer=json.loads(selected.stdout)
            print("AIOS_USER_MODEL_SERVICE_ANSWER="+json.dumps({"upstream_exit":selected.returncode,"answer":service_answer}),flush=True)
            if selected.returncode!=0 or service_answer["state"]!="completed" or service_answer["error"] is not None or service_answer["mutation_performed"]:
                raise RuntimeError("installed scoped service question failed")
            result=service_answer["output"]
            native=[e for e in result["evidence"] if e["data"].get("unit_name")=="sshd.service"]
            if (result["response"]["kind"]!="answer" or result["tool_calls"]<1 or result["tool_calls"]>12
                or not native or native[-1]["data"]["active_state"]!="active"
                or not set(result["response"]["evidence_ids"]).intersection(native[-1]["evidence_ids"])):
                raise RuntimeError("installed service answer lacks scoped native evidence")
        elif question.returncode != 1 or answer["error"] != "MODEL_UNAVAILABLE" or answer["mutation_performed"]:
            raise RuntimeError("unavailable model was not reported truthfully")
        inspection = cli("inspect", "service", "sshd.service", "--json")
        observed = json.loads(inspection.stdout)
        if inspection.returncode != 0 or observed["status"] != "ok" or observed["data"]["boot_id"] != Path("/proc/sys/kernel/random/boot_id").read_text().strip():
            raise RuntimeError("hardened service inspection failed")
        ctl("restart", UNIT)
        after = show()
        if after["ActiveState"] != "active" or after["MainPID"] == properties["MainPID"]:
            raise RuntimeError("packaged restart did not establish a new daemon")
        history_proof=None
        history_client=None
        def verify_bus_owner_pid():
            current=show()
            if current["ActiveState"]!="active" or current["MainPID"]!=after["MainPID"] or not owns_fragment(current):
                raise RuntimeError("installed broker changed during history qualification")
            result=subprocess.run(["busctl","--user","call","org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus","GetConnectionUnixProcessID","s","org.aios.Session1"],env=env,check=True,capture_output=True,timeout=5)
            if result.stdout.decode().strip()!= "u "+after["MainPID"]:
                raise RuntimeError("public bus owner is not the installed unit process")
            return current["MainPID"]
        if installed_model:
            # Compile only a read-only client from this job's immutable source.
            # The broker/model remain the protected current-system binaries above.
            release=Path(__file__).resolve().parents[2]
            arguments=["nix","build","--json","--no-link","--no-update-lock-file","--no-write-lock-file","path:"+str(release)+"#aios-cli"]
            print("AIOS_HISTORY_CLIENT_BUILD="+json.dumps(arguments),flush=True)
            built=subprocess.run(arguments,check=True,stdout=subprocess.PIPE,timeout=900)
            outputs=json.loads(built.stdout)
            if not isinstance(outputs,list) or len(outputs)!=1: raise RuntimeError("unexpected history client build outputs")
            package=Path(outputs[0]["outputs"]["out"])
            history_client=package/"bin/aios-session-history-check"
            info=history_client.stat()
            if not re.fullmatch(r"/nix/store/[a-z0-9]{32}-aios-cli-[A-Za-z0-9._+-]+",str(package)) or package.resolve()!=package or not stat.S_ISREG(info.st_mode) or info.st_uid!=0 or info.st_mode&0o222:
                raise RuntimeError("history test client is not a protected source-built executable")
            installed_owner_pid=verify_bus_owner_pid()
            result=subprocess.run([str(history_client),"--json"],env=env,capture_output=True,timeout=320)
            if verify_bus_owner_pid()!=installed_owner_pid: raise RuntimeError("history broker owner fence changed")
            print("AIOS_PUBLIC_HISTORY_CLIENT="+json.dumps({"client":str(history_client),"client_source_built":True,"broker_installed":True,"upstream_exit":result.returncode,"stdout":result.stdout.decode(),"stderr":result.stderr.decode()}),flush=True)
            if result.returncode!=0: raise RuntimeError("installed public history qualification failed")
            history_proof=json.loads(result.stdout)
            if history_proof.get("evidence_kind")!="actual-installed-public-bus-history-with-readonly-test-client" or history_proof.get("mutation_performed") is not False:
                raise RuntimeError("public history proof class mismatch")
        print("AIOS_USER_SERVICE=" + json.dumps({"outputs":[str(p) for p in paths], "before":properties, "after":after,
            "unit_name":UNIT,"package_unit_sha256":hashlib.sha256(unit_bytes).hexdigest(),"exact_unit_bytes":True,
            "binary_source":"installed-system-closure" if installed_model else "nix-built-packages",
            "executables":{k:{"path":str(v),"sha256":hashlib.sha256(v.read_bytes()).hexdigest()} for k,v in binaries.items()},
            "capabilities":capabilities,"ui_selection_denial":ui_denial,"installed_model":installed_model,"model_answer":answer,"model_service_answer":service_answer,"history_test_client":str(history_client) if history_client else None,"public_history":history_proof,"service_observation":observed}), flush=True)
    except Exception:
        print("AIOS_USER_SERVICE_FAILURE=" + json.dumps(show()), flush=True)
        journal = subprocess.run(["journalctl", "--user", "--user-unit=" + UNIT, "--boot", "--lines=20", "--no-pager", "--output=json", "--output-fields=MESSAGE,PRIORITY,_BOOT_ID,_UID"],
                                 env=env, stdout=subprocess.PIPE, timeout=5)
        print("AIOS_OWN_UNIT_JOURNAL=" + journal.stdout.decode(errors="replace"), flush=True)
        raise
    finally:
        # Only the exact newly copied package unit belongs to this fixture.
        if owns_link():
            properties = show()
            if owns_fragment(properties):
                ctl("stop", UNIT)
                ctl("reset-failed", UNIT, check=False)
                if show()["MainPID"] != "0":
                    raise RuntimeError("owned service did not stop; cleanup refused")
            else:
                raise RuntimeError("runtime service fragment changed; cleanup refused")
            runtime_link.unlink()
            ctl("daemon-reload")
        else:
            raise RuntimeError("runtime service ownership changed; cleanup refused")


if __name__ == "__main__":
    main()
