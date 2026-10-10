# Synthetic disposable desktop only. Never import into a production image.
{ config, pkgs, lib, aiosBaseSystem, ... }:
let
  lifecycle = pkgs.writeShellScriptBin "aios-service-lifecycle-preflight" ''
    exec ${pkgs.python3}/bin/python3 -I ${../../tools/guest/service_lifecycle_preflight.py}
  '';
  builderQualification = pkgs.writeShellScriptBin "aios-builder-preflight" ''
    exec ${pkgs.python3}/bin/python3 -I ${../../tools/guest/builder_preflight.py}
  '';
in {
  # Keep the production-shaped default baseline closure available so this
  # disposable image qualifies candidate evaluation/building, not cache access.
  system.extraDependencies = [ aiosBaseSystem ];
  imports = [ ./aios-graph-test.nix ];
  services.displayManager.autoLogin = { enable = lib.mkForce true; user = "tester"; };
  services.displayManager.defaultSession = "plasma";
  users.users.tester.extraGroups = lib.mkForce [];
  # A separate test-only administrator supplies a real credential to the
  # native polkit agent. Production excludes this account and its fixed hash.
  users.users."approval-test" = {
    isNormalUser = true;
    uid = 1002;
    extraGroups = [ "wheel" ];
    hashedPassword = "$6$minnerite$ngxaoZ/FrRSdynnkKvyoGkm3aE5Tj8k0FpltmBncOH5m4pFbwVdzN8ADt1VbFY0NyLo7PUrmNHxDwv.IBF0qm1";
    openssh.authorizedKeys.keys = config.users.users.dev.openssh.authorizedKeys.keys;
  };
  services.aios.users = lib.mkForce [ "dev" "tester" "approval-test" ];
  # Three real subjects exercise unprivileged IPC and native authorization.
  # Only the enrolled public key enters this disposable image.
  users.users.tester.openssh.authorizedKeys.keys = config.users.users.dev.openssh.authorizedKeys.keys;
  services.openssh.settings.AllowUsers = lib.mkForce [ "dev" "tester" "approval-test" ];
  environment.etc."aios/desktop-test-profile".text = "synthetic-disposable-plasma-wayland-v1\n";
  # Disposable storage qualification fixture. It is visible only in the test
  # image, owned by the normal development caller and deliberately filled.
  fileSystems."/mnt/aios-storage-fixture" = {
    device = "tmpfs";
    fsType = "tmpfs";
    options = [ "size=1M" "mode=0700" "uid=1000" "gid=100" ];
  };
  systemd.services.aios-storage-full-fixture = {
    wantedBy = [ "multi-user.target" ];
    before = [ "aios-execd.service" ];
    unitConfig.RequiresMountsFor = [ "/mnt/aios-storage-fixture" ];
    serviceConfig = {
      Type = "oneshot";
      User = "dev";
      Group = config.users.users.dev.group;
      RemainAfterExit = true;
      ExecStart = pkgs.writeShellScript "fill-aios-storage-fixture" ''
        ${pkgs.coreutils}/bin/dd if=/dev/zero of=/mnt/aios-storage-fixture/full.bin bs=4096 count=1024 status=none || true
        ${pkgs.coreutils}/bin/sync -f /mnt/aios-storage-fixture/full.bin || true
      '';
      NoNewPrivileges = true;
      PrivateNetwork = true;
      ProtectSystem = "strict";
      ReadWritePaths = [ "/mnt/aios-storage-fixture" ];
      CapabilityBoundingSet = "";
    };
  };
  systemd.services.aios-execd = {
    after = [ "aios-storage-full-fixture.service" ];
    requires = [ "aios-storage-full-fixture.service" ];
  };
  systemd.services.aios-removable-device-fixture = {
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${pkgs.systemd}/bin/udevadm trigger --action=change /sys/class/block/sr0";
      NoNewPrivileges = true;
      PrivateNetwork = true;
      ProtectSystem = "strict";
      CapabilityBoundingSet = "";
    };
    unitConfig.ConditionPathExists = "/sys/class/block/sr0";
  };
  systemd.timers.aios-removable-device-fixture = {
    wantedBy = [ "timers.target" ];
    timerConfig = {
      OnBootSec = "1s";
      OnUnitActiveSec = "1s";
      Unit = "aios-removable-device-fixture.service";
    };
  };
  systemd.paths.aios-builder-qualification = {
    wantedBy = [ "multi-user.target" ];
    pathConfig = {
      PathExists = "/tmp/aios-builder-request";
      Unit = "aios-builder-qualification.service";
    };
  };
  systemd.services.aios-builder-qualification = {
    description = "Disposable Minnerite candidate builder qualification";
    after = [ "aios-build.service" ];
    requires = [ "aios-build.service" ];
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${builderQualification}/bin/aios-builder-preflight";
      RuntimeDirectory = "aios-builder-qualification";
      RuntimeDirectoryMode = "0755";
      RuntimeDirectoryPreserve = "yes";
      UMask = "0077";
      NoNewPrivileges = true;
      # This fixed test-only root fixture reads a dev-owned request and connects
      # to the builder-owned root-only socket. The worker retains build authority.
      CapabilityBoundingSet = [ "CAP_DAC_OVERRIDE" "CAP_DAC_READ_SEARCH" ];
      AmbientCapabilities = [ "CAP_DAC_OVERRIDE" "CAP_DAC_READ_SEARCH" ];
      PrivateNetwork = true;
      PrivateTmp = false;
      ProtectSystem = "strict";
      ProtectHome = true;
      ProtectKernelTunables = true;
      ProtectKernelModules = true;
      ProtectKernelLogs = true;
      ProtectControlGroups = true;
      RestrictAddressFamilies = [ "AF_UNIX" ];
      RestrictNamespaces = true;
      RestrictSUIDSGID = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [ "@system-service" ];
      ReadWritePaths = [ "/tmp" "/run/aios-builder-qualification" ];
      TimeoutStartSec = 1380;
      TasksMax = 16;
      MemoryMax = "128M";
    };
  };
  systemd.paths.aios-service-lifecycle-test = {
    wantedBy = [ "multi-user.target" ];
    pathConfig = {
      PathExists = "/tmp/aios-service-lifecycle-request";
      Unit = "aios-service-lifecycle-test.service";
    };
  };
  systemd.services.aios-service-lifecycle-test = {
    description = "Disposable Minnerite service lifecycle qualification";
    after = [ "graphical.target" "aios-state.service" "aios-observer.service" "aios-build.service" "aios-execd.service" ];
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${lifecycle}/bin/aios-service-lifecycle-preflight";
      RuntimeDirectory = "aios-service-lifecycle";
      RuntimeDirectoryMode = "0755";
      RuntimeDirectoryPreserve = "yes";
      UMask = "0077";
      NoNewPrivileges = true;
      # Fixed test-only root orchestration must read the dev-owned request and
      # tester bus, enter the tester identity for its user manager, and connect
      # to the builder-owned root-only status socket.
      CapabilityBoundingSet = [ "CAP_DAC_OVERRIDE" "CAP_DAC_READ_SEARCH" "CAP_SETUID" "CAP_SETGID" ];
      AmbientCapabilities = [ "CAP_DAC_OVERRIDE" "CAP_DAC_READ_SEARCH" "CAP_SETUID" "CAP_SETGID" ];
      PrivateNetwork = true;
      PrivateTmp = false;
      ProtectSystem = "strict";
      ProtectHome = "read-only";
      ProtectKernelTunables = true;
      ProtectKernelModules = true;
      ProtectKernelLogs = true;
      ProtectControlGroups = true;
      RestrictAddressFamilies = [ "AF_UNIX" ];
      RestrictNamespaces = true;
      RestrictSUIDSGID = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
      SystemCallFilter = [ "@system-service" ];
      ReadWritePaths = [ "/tmp" "/run/aios-service-lifecycle" ];
      TimeoutStartSec = 120;
      TasksMax = 32;
      MemoryMax = "128M";
    };
  };
  environment.systemPackages = [
    lifecycle
    builderQualification
    (pkgs.writeScriptBin "aios-desktop-test-probe" ''
      #!${pkgs.runtimeShell}
      exec ${pkgs.python3}/bin/python3 -I ${../../tools/guest/desktop_probe.py}
    '')
  ];
}
