# Synthetic disposable desktop only. Never import into a production image.
{ config, pkgs, lib, ... }: {
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
      RemainAfterExit = true;
      ExecStart = pkgs.writeShellScript "fill-aios-storage-fixture" ''
        ${pkgs.coreutils}/bin/dd if=/dev/zero of=/mnt/aios-storage-fixture/full.bin bs=4096 count=1024 status=none || true
        ${pkgs.coreutils}/bin/sync -f /mnt/aios-storage-fixture/full.bin
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
  environment.systemPackages = [ (pkgs.writeScriptBin "aios-desktop-test-probe" ''
    #!${pkgs.runtimeShell}
    exec ${pkgs.python3}/bin/python3 -I ${../../tools/guest/desktop_probe.py}
  '') ];
}
