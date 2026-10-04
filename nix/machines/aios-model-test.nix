# Actual model service in a disposable two-user desktop acceptance image.
# No mock model, alternate daemon mode, or model privilege bypass is enabled.
{ aiosModelArtifact, config, pkgs, lib, ... }:
let
  fixture = pkgs.runCommandNoCC "aios-model-acceptance-fixture" {} ''
    mkdir -p "$out"
    install -m444 ${../../tools/guest/model_lifecycle_preflight.py} "$out/model_lifecycle_preflight.py"
    install -m444 ${../../tools/guest/installed_model_lifecycle_smoke.py} "$out/installed_model_lifecycle_smoke.py"
    install -m444 ${../../tools/guest/model_service_smoke.py} "$out/model_service_smoke.py"
    install -m444 ${../../tools/guest/snapshot.py} "$out/snapshot.py"
  '';
  runner = file: pkgs.writeScriptBin file ''
    #!${pkgs.runtimeShell}
    exec ${pkgs.python3}/bin/python3 -I -c ${lib.escapeShellArg "import runpy,sys; sys.path.insert(0,'${fixture}'); runpy.run_path('${fixture}/${file}.py',run_name='__main__')"}
  '';
  preflight = runner "model_lifecycle_preflight";
  probe = runner "installed_model_lifecycle_smoke";
in {
  assertions = [ {
    assertion = config.services.aios.development.enable && config.environment.etc."aios/guest-role".text == "development\n";
    message = "Fixed model crash/restart fixtures are exclusive to the disposable development image.";
  } ];
  services.aios.users = [ "dev" "tester" ];
  services.aios.model = {
    enable = true;
    manifest = "${aiosModelArtifact}/lock.json";
  };
  environment.etc."aios/model-test-profile".text = "installed-normal-cpu-model-v1\n";
  environment.systemPackages = [ probe ];
  # No RPC or sudo route. Only this compiled initial test-image unit can invoke
  # the root fixture; it names one fixed service and accepts no arguments.
  systemd.services.aios-model-acceptance = {
    description = "Fixed initial Horizon OS model crash/restart acceptance";
    wantedBy = [ "multi-user.target" ];
    after = [ "sshd.service" "aios-model.socket" ];
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
      NoNewPrivileges = true;
      # Fixed process PSS observation and fixed-unit SIGKILL are test-only.
      CapabilityBoundingSet = [ "CAP_KILL" "CAP_SYS_PTRACE" ];
      PrivateNetwork = true;
      RestrictAddressFamilies = "AF_UNIX";
      PrivateTmp = true;
      ProtectSystem = "strict";
      ProtectHome = true;
    };
  };
}
