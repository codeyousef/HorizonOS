# Actual model service in a disposable desktop and five-user load image.
# No mock model, alternate daemon mode, or model privilege bypass is enabled.
{ aiosModelArtifact, config, pkgs, lib, ... }:
let
  lock = builtins.fromJSON (builtins.readFile ../../models/lock.json);
  corrupt = pkgs.runCommandNoCC "aios-model-corruption-test" { nativeBuildInputs = [ pkgs.python3 ]; } ''
    mkdir -p "$out"
    python3 - ${aiosModelArtifact}/${lock.artifact.filename} "$out/${lock.artifact.filename}" "$out/profile.json" <<'PY'
    import hashlib,json,os,sys
    original,changed,profile=sys.argv[1:]
    with open(original,'rb') as source,open(changed,'wb') as target:
        first=source.read(1); target.write(bytes([first[0]^1]))
        while data:=source.read(1024*1024): target.write(data)
    os.chmod(changed,0o444)
    def digest(path):
        h=hashlib.sha256()
        with open(path,'rb') as source:
            while data:=source.read(1024*1024): h.update(data)
        return h.hexdigest()
    expected='${lock.artifact.sha256}'
    assert os.stat(original).st_size==os.stat(changed).st_size==${toString lock.artifact.bytes}
    assert digest(original)==expected and digest(changed)!=expected
    with open(profile,'w') as target:
        json.dump({'schema_version':1,'original':original,'corrupt':changed,'bytes':os.stat(original).st_size,'expected_sha256':expected,'corrupt_sha256':digest(changed)},target)
    os.chmod(profile,0o444)
    PY
  '';
  fixture = pkgs.runCommandNoCC "aios-model-acceptance-fixture" {} ''
    mkdir -p "$out"
    install -m444 ${../../tools/guest/model_lifecycle_preflight.py} "$out/model_lifecycle_preflight.py"
    install -m444 ${../../tools/guest/installed_model_lifecycle_smoke.py} "$out/installed_model_lifecycle_smoke.py"
    install -m444 ${../../tools/guest/model_service_smoke.py} "$out/model_service_smoke.py"
    install -m444 ${../../tools/guest/model_queue_fixture.py} "$out/model_queue_fixture.py"
    install -m444 ${../../tools/guest/model_public_fixture.py} "$out/model_public_fixture.py"
    install -m444 ${../../tools/guest/service_failure_fixture.py} "$out/service_failure_fixture.py"
    install -m444 ${../../tools/guest/desktop_probe.py} "$out/desktop_probe.py"
    install -m444 ${../../tools/guest/snapshot.py} "$out/snapshot.py"
  '';
  runner = file: pkgs.writeScriptBin file ''
    #!${pkgs.runtimeShell}
    exec ${pkgs.python3}/bin/python3 -I -c ${lib.escapeShellArg "import runpy,sys; sys.path.insert(0,'${fixture}'); runpy.run_path('${fixture}/${file}.py',run_name='__main__')"}
  '';
  preflight = runner "model_lifecycle_preflight";
  probe = runner "installed_model_lifecycle_smoke";
  failedService = pkgs.writeShellScript "aios-service-failure-fixture" ''
    ${pkgs.coreutils}/bin/sleep 8
    exit 7
  '';
  restartingService = pkgs.writeShellScript "aios-service-restart-fixture" ''
    ${pkgs.coreutils}/bin/sleep 1
    exit 7
  '';
  failureSandbox = {
    User = "nobody";
    NoNewPrivileges = true;
    CapabilityBoundingSet = "";
    ProtectSystem = "strict";
    ProtectHome = true;
    PrivateTmp = true;
    PrivateNetwork = true;
    RestrictAddressFamilies = [ "AF_UNIX" ];
    MemoryMax = "32M";
    TasksMax = 8;
    TimeoutStartSec = 15;
    TimeoutStopSec = 5;
  };
in {
  assertions = [ {
    assertion = config.services.aios.development.enable && config.environment.etc."aios/guest-role".text == "development\n";
    message = "Fixed model crash/restart fixtures are exclusive to the disposable development image.";
  } ];
  services.aios.users = [ "dev" "tester" "model-load-a" "model-load-b" "model-load-c" ];
  users.users = builtins.listToAttrs (lib.imap0 (index: name: {
    inherit name;
    value = {
      isNormalUser = true;
      uid = 1100 + index;
      createHome = false;
      home = "/var/empty";
      hashedPassword = "!";
      shell = "${pkgs.shadow}/bin/nologin";
      openssh.authorizedKeys.keys = [];
    };
  }) [ "model-load-a" "model-load-b" "model-load-c" ]);
  services.aios.model = {
    enable = true;
    manifest = "${aiosModelArtifact}/lock.json";
  };
  environment.etc."aios/model-test-profile".text = "installed-normal-cpu-model-v1\n";
  environment.etc."aios/service-fixture-profile".text = "fixed-native-service-failures-v1\n";
  environment.etc."aios/model-corruption-test.json" = { source = "${corrupt}/profile.json"; mode = "symlink"; };
  systemd.tmpfiles.rules = [ "d /run/systemd/system/aios-model.service.d 0700 root root -" ];
  environment.systemPackages = [ probe ];
  # Deliberately failing, unprivileged units are never enabled at normal boot.
  # Only the fixed initial coordinator starts them in this disposable image.
  systemd.services.aios-service-failure-fixture = {
    after = [ "sshd.service" ];
    serviceConfig = failureSandbox // { Type = "oneshot"; ExecStart = failedService; Restart = "no"; };
  };
  systemd.services.aios-service-restart-fixture = {
    after = [ "sshd.service" ];
    unitConfig = { StartLimitIntervalSec = 30; StartLimitBurst = 3; };
    serviceConfig = failureSandbox // { Type = "simple"; ExecStart = restartingService; Restart = "on-failure"; RestartSec = 1; };
  };
  # No RPC or sudo route. Only this compiled initial test-image unit can invoke
  # the root fixture; it names one fixed service and accepts no arguments.
  systemd.services.aios-model-acceptance = {
    description = "Fixed initial Minnerite model crash/restart acceptance";
    wantedBy = [ "multi-user.target" ];
    after = [ "sshd.service" "aios-model.socket" "user@1001.service" ];
    requires = [ "user@1001.service" ];
    serviceConfig = {
      # Exec startup completes immediately; inference tests must not hold the
      # multi-user/graphical target until their measurements finish.
      Type = "exec";
      ExecStart = "${preflight}/bin/model_lifecycle_preflight";
      RuntimeDirectory = "aios-model-acceptance";
      RuntimeDirectoryMode = "0755";
      RuntimeDirectoryPreserve = "yes";
      UMask = "0077";
      TimeoutStartSec = 180;
      RuntimeMaxSec = 180;
      NoNewPrivileges = true;
      # SETUID/GID only permanently drop forked clients to fixed normal users.
      # Fixed process PSS observation and fixed-unit SIGKILL are test-only.
      CapabilityBoundingSet = [ "CAP_KILL" "CAP_SYS_PTRACE" "CAP_SETUID" "CAP_SETGID" ];
      PrivateNetwork = true;
      RestrictAddressFamilies = "AF_UNIX";
      PrivateTmp = true;
      ProtectSystem = "strict";
      # Fixed tester bus/runtime and one root-only test mount drop-in directory.
      # No home contents, arbitrary unit file or public privileged RPC.
      ProtectHome = "tmpfs";
      BindPaths = [ "/run/user/1001" ];
      ReadWritePaths = [ "/run/systemd/system/aios-model.service.d" ];
    };
  };
}
