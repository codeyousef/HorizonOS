# Synthetic disposable desktop only. Never import into a production image.
{ pkgs, lib, ... }: {
  services.displayManager.autoLogin = { enable = lib.mkForce true; user = "tester"; };
  services.displayManager.defaultSession = "plasma";
  users.users.tester.extraGroups = lib.mkForce [];
  environment.etc."aios/desktop-test-profile".text = "synthetic-disposable-plasma-wayland-v1\n";
  environment.systemPackages = [ (pkgs.writeScriptBin "aios-desktop-test-probe" ''
    #!${pkgs.runtimeShell}
    exec ${pkgs.python3}/bin/python3 -I ${../../tools/guest/desktop_probe.py}
  '') ];
}
