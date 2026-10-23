{ pkgs, extract, fixtureUuid, installationUuid }:
let
  python = pkgs.python3.withPackages (p: [ p.reportlab p.pillow ]);
  client = pkgs.writeText "extract-corpus-client.py" (builtins.readFile ./extract_client.py);
  gate = pkgs.writeText "extract-fixture-identity.py" (builtins.readFile ./scope_fixture_identity.py);
  memoryProbe = pkgs.writeText "extract-memory-probe.py" ''
    import os
    if os.fork() == 0:
        chunks = []
        while True:
            chunks.append(bytearray(8 * 1024 * 1024))
    else:
        os.wait()
  '';
  cpuProbe = pkgs.writeText "extract-cpu-probe.py" ''
    import os
    if os.fork() == 0:
        while True:
            pass
    else:
        os.wait()
  '';
in pkgs.testers.runNixOSTest {
  name = "aios-extract-native";
  requiredFeatures.kvm = false;
  node.specialArgs.aiosExtract = extract;
  nodes.machine = { config, ... }: {
    imports = [ ../../nix/modules/aios ];
    services.aios.extract.enable = true;
    virtualisation.memorySize = 2048;
    virtualisation.cores = 2;
    virtualisation.diskSize = 1024;
    virtualisation.qemu.forceAccel = false;
    virtualisation.qemu.options = [ "-uuid ${fixtureUuid}" "-machine accel=tcg" ];
    # The test-driver default panics even on a deliberately contained cgroup
    # OOM. Exercise normal worker OOM recovery in this disposable node only.
    boot.kernel.sysctl."vm.panic_on_oom" = 0;
    users.users.alice = { isNormalUser = true; uid = 1000; };
    services.openssh.enable = true;
    environment.systemPackages = [ extract python pkgs.openssh ];
    environment.etc."aios/scope-qualification.json".text = builtins.toJSON {
      schema_version = 1; dmi_uuid = fixtureUuid; installation_uuid = installationUuid;
      guest_role = "development"; management_channel = "nixos-test-driver";
    };
    systemd.user.services.extract-corpus = {
      serviceConfig = { Type = "oneshot"; RemainAfterExit = true; TimeoutStartSec = 300; ExecStart = "${python}/bin/python3 ${client} corpus"; };
    };
    # Probe the exact production whole-tree resource profile. Only the fixed
    # executable/input lifecycle differs; these are labelled test probes.
    systemd.user.services.extract-memory-probe.serviceConfig = config.systemd.user.services."aios-extract@".serviceConfig // {
      StandardInput = "null";
      ExecStart = "${python}/bin/python3 ${memoryProbe}";
    };
    systemd.user.services.extract-cpu-probe.serviceConfig = config.systemd.user.services."aios-extract@".serviceConfig // {
      StandardInput = "null";
      ExecStart = "${python}/bin/python3 ${cpuProbe}";
    };
  };
  testScript = ''
    import json, shlex, re
    machine.start()
    machine.wait_for_unit("sshd.service")
    gate = "${pkgs.python3}/bin/python3 ${gate}"
    identity = json.loads(machine.succeed(gate + " record"))
    def checked(command):
        return machine.succeed(gate + " verify >/dev/null && ( set -e; " + command + " )")
    def user_command(command):
        return checked("su -s /run/current-system/sw/bin/sh alice -c " + shlex.quote("export XDG_RUNTIME_DIR=/run/user/1000; export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus; " + command))
    checked("loginctl enable-linger alice; systemctl start user@1000.service")
    user_command("systemctl --user is-active aios-extract.socket")
    user_command("${python}/bin/python3 ${client} generate")
    user_command("systemctl --user start --no-block extract-corpus.service")
    try:
        machine.wait_for_file("/home/alice/extract-test/worker.json", timeout=30)
        # Observe the actual installed worker from outside its PID/user/mount
        # namespaces, before giving it permission to finish the fixture request.
        program = 'import json,os,re; from pathlib import Path; w=json.loads(Path("/home/alice/extract-test/worker.json").read_text()); p=w["pid"]; assert type(p)==int and p>1; s=dict(line.split(":",1) for line in Path(f"/proc/{p}/status").read_text().splitlines() if ":" in line); exe=str(Path(f"/proc/{p}/exe").resolve(strict=True)); assert exe=="${extract}/bin/aios-extract"; assert s["Uid"].split()==["1000"]*4; assert s["NoNewPrivs"].strip()=="1" and s["Seccomp"].strip()=="2"; assert all(int(s[k].strip(),16)==0 for k in ["CapEff","CapPrm","CapInh","CapAmb","CapBnd"]); assert s["NSpid"].split()[-1]=="1"; assert all(os.readlink(f"/proc/{p}/ns/{n}")!=os.readlink(f"/proc/self/ns/{n}") for n in ["pid","user","mnt","net"]); out=dict(pid=p,unit=w["unit"],executable=exe,uid=1000,nnp=1,seccomp=2,capabilities_empty=True,private_namespaces=True); Path("/home/alice/extract-test/inspected").touch(); print(json.dumps(out))'
        security = json.loads(checked("${pkgs.python3}/bin/python3 -c " + shlex.quote(program)))
        assert re.fullmatch(r"aios-extract@[^/]+\.service", security["unit"])
        machine.wait_for_unit("extract-corpus.service", user="alice", timeout=300)
        result = json.loads(checked("cat /home/alice/extract-test/result.json"))
    except Exception:
        print(user_command("journalctl --user -u 'aios-extract@*' -u extract-corpus.service --no-pager -n 40"))
        raise
    probes = {}
    assert checked("cat /proc/sys/vm/panic_on_oom").strip() == "0"
    for name, expected in [("memory", "oom-kill"), ("cpu", "timeout")]:
        unit = "extract-" + name + "-probe.service"
        # A failing bounded probe is expected; its real terminal result is
        # checked instead of turning an ignored command failure into success.
        user_command("systemctl --user start --no-block " + unit)
        machine.wait_until_succeeds(gate + " verify >/dev/null && su -s /run/current-system/sw/bin/sh alice -c " + shlex.quote("export XDG_RUNTIME_DIR=/run/user/1000; export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus; test $(systemctl --user show " + unit + " -p ActiveState --value) = failed"), timeout=45)
        data = dict(line.split("=",1) for line in user_command("systemctl --user show " + unit + " -p Result -p MemoryMax -p MemorySwapMax -p RuntimeMaxUSec -p CPUUsageNSec -p CPUQuotaPerSecUSec -p TasksMax -p KillMode").splitlines())
        assert data["Result"] == expected
        assert data["MemoryMax"] == "536870912" and data["MemorySwapMax"] == "0"
        assert data["CPUQuotaPerSecUSec"] == "900ms"
        assert data["RuntimeMaxUSec"] == "10s" and data["TasksMax"] == "8" and data["KillMode"] == "control-group"
        if name == "cpu":
            assert 0 < int(data["CPUUsageNSec"]) < 10_000_000_000
        probes[name] = data
    user_command("${python}/bin/python3 ${client} recover")
    result.update(identity=identity, worker_security=security, resource_probes=probes,
                  resource_profile_probes_qualified=True, caller_survived_resource_failure=True)
    with open(machine.out_dir / "extract.json", "w") as report:
        json.dump(result, report, indent=2)
  '';
}
