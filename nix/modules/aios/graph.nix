{ config, lib, pkgs, aiosState ? null, ... }:
let
  cfg = config.services.aios.graph;
  common = {
    User = "aios-state"; Group = "aios-state"; UMask = "0077";
    NoNewPrivileges = true; CapabilityBoundingSet = "";
    ProtectSystem = "strict"; ProtectHome = "tmpfs";
    PrivateTmp = true; PrivateDevices = true; PrivateNetwork = true;
    ProtectKernelTunables = true; ProtectKernelModules = true;
    ProtectKernelLogs = true; ProtectControlGroups = true;
    RestrictAddressFamilies = [ "AF_UNIX" ]; RestrictNamespaces = true;
    RestrictSUIDSGID = true; LockPersonality = true;
    MemoryDenyWriteExecute = true; SystemCallArchitectures = "native";
    SystemCallFilter = [ "@system-service" ];
    InaccessiblePaths = [ "-/var/lib/aios/transactions" "-/nix/var/nix/daemon-socket" ];
    MemoryMax = "256M"; TasksMax = 32; LimitNOFILE = 2048;
    TimeoutStartSec = 15; TimeoutStopSec = 5;
  };
in {
  options.services.aios.graph.enable = lib.mkEnableOption "the private native system graph owner";
  config = lib.mkIf cfg.enable {
    assertions = [ { assertion = aiosState != null; message = "Native graph ownership requires the packaged aios-state binaries."; } ];
    users.groups.aios-state = {};
    users.users.aios-state = { isSystemUser = true; group = "aios-state"; home = "/var/empty"; };
    environment.systemPackages = lib.optional (aiosState != null) aiosState;
    systemd.services.aios-state = {
      description = "Horizon OS private native system graph";
      wantedBy = [ "multi-user.target" ]; after = [ "local-fs.target" "dbus.service" ];
      serviceConfig = common // {
        Type = "exec"; ExecStart = "${aiosState}/bin/aios-stated";
        StateDirectory = "aios/state"; StateDirectoryMode = "0700";
        RuntimeDirectory = "aios-state"; RuntimeDirectoryMode = "0700";
        Restart = "on-failure"; RestartSec = 2;
      };
    };
    systemd.services.aios-reconcile = {
      description = "Horizon OS native graph reconciliation";
      after = [ "aios-state.service" ];
      serviceConfig = common // { Type = "oneshot"; ExecStart = "${aiosState}/bin/aios-stated --reconcile"; };
    };
    systemd.timers.aios-reconcile = {
      description = "Reconcile the Horizon OS graph every fifteen minutes";
      wantedBy = [ "timers.target" ];
      timerConfig = { OnActiveSec = "15min"; OnUnitActiveSec = "15min"; AccuracySec = "1s"; Unit = "aios-reconcile.service"; };
    };
    systemd.user.services.aios-user-profile-reconcile = {
      description = "Horizon OS private per-user Nix profile reconciliation";
      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${aiosState}/bin/aios-user-stated --reconcile";
        UMask = "0077";
        StateDirectory = "aios/user-graph";
        StateDirectoryMode = "0700";
        # User-manager mount-namespace protections hide the fixed boot-id and
        # profile paths this read-only provider must compare. UID DAC, no
        # capabilities and the syscall/address-family filters remain the bound.
        NoNewPrivileges = true;
        CapabilityBoundingSet = "";
        RestrictAddressFamilies = [ "AF_UNIX" ];
        RestrictNamespaces = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [ "@system-service" ];
        MemoryMax = "128M";
        TasksMax = 16;
        LimitNOFILE = 1024;
        TimeoutStartSec = 15;
      };
    };
    systemd.user.timers.aios-user-profile-reconcile = {
      description = "Reconcile the private per-user Nix profile graph";
      wantedBy = [ "timers.target" ];
      timerConfig = {
        OnActiveSec = "1min";
        OnUnitActiveSec = "15min";
        AccuracySec = "1s";
        Unit = "aios-user-profile-reconcile.service";
      };
    };
  };
}
