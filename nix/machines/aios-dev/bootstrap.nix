{ pkgs, lib, ... }:
let
  guestUUID = lib.removeSuffix "\n" (builtins.readFile ./guest.uuid);
  installationUUID = lib.removeSuffix "\n" (builtins.readFile ./installation.uuid);
  identity = pkgs.writeScriptBin "aios-guest-identity" ''
    #!${pkgs.runtimeShell}
    exec ${pkgs.python3}/bin/python3 ${./identity.py}
  '';
  serviceUsers = [ "aios-state" "aios-observer" "aios-builder" "aios-model" ];
in {
  nixpkgs.hostPlatform = "x86_64-linux";
  system.stateVersion = "26.05";
  networking.hostName = "aios-dev";
  networking.useDHCP = lib.mkDefault true;
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  fileSystems."/boot".options = lib.mkForce [ "fmask=0077" "dmask=0077" ];
  boot.kernelParams = [ "console=tty0" "console=ttyS0,115200n8" ];
  nix.settings = { experimental-features = [ "nix-command" "flakes" ]; trusted-users = [ "root" ]; sandbox = true; };
  users.users = lib.genAttrs serviceUsers (name: { isSystemUser = true; group = name; }) // {
    dev = { isNormalUser = true; extraGroups = []; openssh.authorizedKeys.keys = [ (lib.removeSuffix "\n" (builtins.readFile ./dev.pub)) ]; };
    tester = { isNormalUser = true; extraGroups = [ "wheel" ]; };
  };
  users.groups = lib.genAttrs serviceUsers (_: {});
  services.openssh = {
    enable = true;
    hostKeys = [ { path = "/etc/ssh/ssh_host_ed25519_key"; type = "ed25519"; } ];
    settings = {
      PermitRootLogin = "no"; PasswordAuthentication = false; KbdInteractiveAuthentication = false;
      AllowUsers = [ "dev" ]; AllowAgentForwarding = false; AllowTcpForwarding = "no";
      AllowStreamLocalForwarding = "no"; PermitTunnel = "no"; X11Forwarding = false;
    };
  };
  environment.systemPackages = with pkgs; [ identity python3 git cargo rustc rustfmt clippy btrfs-progs ];
  environment.etc."aios/installation-uuid".text = installationUUID + "\n";
  environment.etc."aios/expected-dmi-uuid".text = guestUUID + "\n";
  environment.etc."aios/guest-role".text = "development\n";
  systemd.services.aios-bootstrap-identity = {
    description = "Publish observed VM DMI identity without exposing privileged files";
    wantedBy = [ "multi-user.target" ];
    before = [ "sshd.service" ];
    serviceConfig.Type = "oneshot";
    script = ''
      install -d -m 0755 /run/aios
      observed=$(tr '[:upper:]' '[:lower:]' </sys/class/dmi/id/product_uuid)
      test "$observed" = ${lib.escapeShellArg guestUUID}
      printf '%s\n' "$observed" >/run/aios/dmi-uuid
      chmod 0444 /run/aios/dmi-uuid
    '';
  };
  systemd.services.sshd = {
    requires = [ "aios-bootstrap-identity.service" ];
    after = [ "aios-bootstrap-identity.service" ];
  };
  # AIOS services and development deployment authority are not enabled here.
  # No desktop autologin, plaintext passwords, host secrets or host mounts.
}
