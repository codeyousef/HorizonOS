# Synthetic disposable desktop only. Never import into a production image.
{ config, pkgs, lib, ... }:
let
  lifecycle = pkgs.writeShellScriptBin "aios-service-lifecycle-preflight" ''
    exec ${pkgs.python3}/bin/python3 -I ${../../tools/guest/service_lifecycle_preflight.py}
  '';
in {
  imports = [ ./aios-graph-test.nix ];
  services.displayManager.autoLogin = { enable = lib.mkForce true; user = "tester"; };
  services.displayManager.defaultSession = "plasma";
  users.users.tester.extraGroups = lib.mkForce [];
  # Two real unprivileged subjects for IPC qualification. Only the enrolled
  # public key enters this disposable image; production excludes both accounts.
  users.users.tester.openssh.authorizedKeys.keys = config.users.users.dev.openssh.authorizedKeys.keys;
  services.openssh.settings.AllowUsers = lib.mkForce [ "dev" "tester" ];
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
  systemd.paths.aios-service-lifecycle-test = {
    wantedBy = [ "multi-user.target" ];
    pathConfig = {
      PathExists = "/tmp/aios-service-lifecycle-request";
      Unit = "aios-service-lifecycle-test.service";
    };
  };
  systemd.services.aios-service-lifecycle-test = {
    description = "Disposable Horizon OS service lifecycle qualification";
    after = [ "graphical.target" "aios-state.service" "aios-execd.service" ];
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${lifecycle}/bin/aios-service-lifecycle-preflight";
      RuntimeDirectory = "aios-service-lifecycle";
      RuntimeDirectoryMode = "0755";
      RuntimeDirectoryPreserve = "yes";
      UMask = "0077";
      NoNewPrivileges = true;
      CapabilityBoundingSet = "";
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
      ReadWritePaths = [ "/tmp" "/run/aios-service-lifecycle" ];
      TimeoutStartSec = 120;
      TasksMax = 32;
      MemoryMax = "128M";
    };
  };
  environment.systemPackages = [
    lifecycle
    (pkgs.writeScriptBin "aios-desktop-test-probe" ''
      #!${pkgs.runtimeShell}
      exec ${pkgs.python3}/bin/python3 -I ${../../tools/guest/desktop_probe.py}
    '')
  ];
}
