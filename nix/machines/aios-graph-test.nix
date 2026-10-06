# Fixed graph failure qualification; imported only by disposable desktop images.
{ config, pkgs, lib, aiosState, ... }:
let
  fixture = pkgs.runCommandNoCC "aios-graph-acceptance-fixture" {} ''
    mkdir -p "$out"
    install -m444 ${../../tools/guest/graph_owner_preflight.py} "$out/graph_owner_preflight.py"
    install -m444 ${../../tools/guest/installed_graph_owner_smoke.py} "$out/installed_graph_owner_smoke.py"
    install -m444 ${../../tools/guest/desktop_probe.py} "$out/desktop_probe.py"
    install -m444 ${../../tools/guest/snapshot.py} "$out/snapshot.py"
  '';
  runner = file: pkgs.writeScriptBin file ''
    #!${pkgs.runtimeShell}
    exec ${pkgs.python3}/bin/python3 -I -c ${lib.escapeShellArg "import runpy,sys; sys.path.insert(0,'${fixture}'); runpy.run_path('${fixture}/${file}.py',run_name='__main__')"}
  '';
in {
  assertions = [ {
    assertion = config.services.aios.development.enable && config.services.aios.graph.enable
      && config.environment.etc."aios/guest-role".text == "development\n"
      && config.environment.etc."aios/desktop-test-profile".text == "synthetic-disposable-plasma-wayland-v1\n";
    message = "Fixed graph failure fixtures are exclusive to the disposable enrolled desktop image.";
  } ];
  environment.etc."aios/graph-test-profile".text = "fixed-installed-graph-owner-v1\n";
  environment.systemPackages = [ (runner "installed_graph_owner_smoke") ];
  systemd.services.aios-graph-acceptance = {
    description = "Horizon OS fixed initial installed graph qualification";
    wantedBy = [ "multi-user.target" ];
    after = [ "aios-state.service" "sshd.service" "user@1001.service" ];
    path = [ pkgs.systemd pkgs.procps ];
    environment.AIOS_GRAPH_EXECUTABLE = "${aiosState}/bin/aios-stated";
    serviceConfig = {
      Type = "exec"; User = "root"; Group = "root";
      ExecStart = "${runner "graph_owner_preflight"}/bin/graph_owner_preflight";
      Restart = "no"; TimeoutStartSec = 1300; RuntimeMaxSec = 1300;
      RuntimeDirectory = "aios-graph-acceptance"; RuntimeDirectoryMode = "0755";
      RuntimeDirectoryPreserve = "yes";
      NoNewPrivileges = true;
      CapabilityBoundingSet = [ "CAP_DAC_OVERRIDE" "CAP_SYS_PTRACE" "CAP_SETUID" "CAP_SETGID" ];
      ProtectSystem = "strict"; ProtectHome = "tmpfs";
      ReadWritePaths = [ "/var/lib/aios/state" ];
      ReadOnlyPaths = [ "-/var/lib/aios/transactions" ];
      PrivateTmp = true; PrivateNetwork = true;
      RestrictAddressFamilies = [ "AF_UNIX" ];
      MemoryMax = "128M"; TasksMax = 32; UMask = "0077";
    };
  };
}
