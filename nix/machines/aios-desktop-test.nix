# Synthetic disposable desktop only. Never import into a production image.
{ config, pkgs, lib, ... }: {
  services.displayManager.autoLogin = { enable = lib.mkForce true; user = "tester"; };
  services.displayManager.defaultSession = "plasma";
  users.users.tester.extraGroups = lib.mkForce [];
  # Two real unprivileged subjects for IPC qualification. Only the enrolled
  # public key enters this disposable image; production excludes both accounts.
  users.users.tester.openssh.authorizedKeys.keys = config.users.users.dev.openssh.authorizedKeys.keys;
  services.openssh.settings.AllowUsers = lib.mkForce [ "dev" "tester" ];
  environment.etc."aios/desktop-test-profile".text = "synthetic-disposable-plasma-wayland-v1\n";
  environment.systemPackages = [ (pkgs.writeScriptBin "aios-desktop-test-probe" ''
    #!${pkgs.runtimeShell}
    exec ${pkgs.python3}/bin/python3 -I ${../../tools/guest/desktop_probe.py}
  '') ];
}
