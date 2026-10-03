{ config, pkgs, lib, aiosPackages, aiosStateContract, ... }:
let
  enrollmentPath = ./enrollment.json;
  enrollment = if builtins.pathExists enrollmentPath
    then builtins.fromJSON (builtins.readFile enrollmentPath)
    else throw "AIOS_DEV_ENROLLMENT_REQUIRED: use devctl build --target system to prepare the enrolled VM candidate.";
  uuid = value: builtins.isString value && builtins.match "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}" value != null;
  keys = [ "schema_version" "dmi_uuid" "installation_uuid" "guest_role" "disk_serial" "management_channel" "authorized_key" ];
  valid = builtins.isAttrs enrollment && builtins.attrNames enrollment == builtins.sort builtins.lessThan keys &&
    builtins.isInt enrollment.schema_version && enrollment.schema_version == 1 && uuid enrollment.dmi_uuid && uuid enrollment.installation_uuid &&
    enrollment.guest_role == "development" && enrollment.disk_serial == "AIOS_DEV_ROOT" &&
    enrollment.management_channel == "ssh-development" &&
    builtins.isString enrollment.authorized_key && builtins.match "ssh-ed25519 [A-Za-z0-9+/=]+" enrollment.authorized_key != null;
  guestUUID = assert valid; enrollment.dmi_uuid;
  installationUUID = assert valid; enrollment.installation_uuid;
  identity = pkgs.writeScriptBin "aios-guest-identity" ''
    #!${pkgs.runtimeShell}
    exec ${pkgs.python3}/bin/python3 ${../../../tools/guest/identity.py}
  '';
  serviceUsers = [ "aios-state" "aios-observer" "aios-builder" "aios-model" ];
in {
  imports = [ ../../modules/aios ../../modules/aios/template.nix (
    assert !(builtins.pathExists ../../../managed.json) || (builtins.pathExists ../../../catalog.json && builtins.readFile ../../../catalog.json == builtins.toJSON aiosStateContract.catalog);
    import ../../state/managed.nix {
      catalog = aiosStateContract.catalog;
      managedJSON = if builtins.pathExists ../../../managed.json then builtins.readFile ../../../managed.json else builtins.toJSON aiosStateContract.defaults;
      installationStateVersion = "26.05";
    }
  ) ];
  assertions = [ { assertion = valid; message = "Development enrollment must match the fixed schema and management identity."; } ];
  nixpkgs.hostPlatform = "x86_64-linux";
  system.stateVersion = "26.05";
  networking.hostName = "aios-dev";
  networking.useDHCP = true;
  boot.initrd.availableKernelModules = [ "virtio_pci" "virtio_blk" "virtio_scsi" "ahci" "sd_mod" ];
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  boot.kernelParams = [ "console=tty0" "console=ttyS0,115200n8" ];
  fileSystems = lib.genAttrs [ "/" "/home" "/nix" "/var" ] (mount: {
    device = "/dev/disk/by-uuid/${installationUUID}";
    fsType = "btrfs";
    options = [ "subvol=@${if mount == "/" then "root" else lib.removePrefix "/" mount}" "compress=zstd" ];
  }) // {
    "/boot" = { device = "/dev/disk/by-label/AIOS_EFI"; fsType = "vfat"; options = [ "fmask=0077" "dmask=0077" ]; };
  };
  nix.settings = { experimental-features = [ "nix-command" "flakes" ]; trusted-users = lib.mkForce [ "root" ]; sandbox = true; };
  users.users = lib.genAttrs serviceUsers (name: { isSystemUser = true; group = name; }) // {
    dev = { isNormalUser = true; extraGroups = []; openssh.authorizedKeys.keys = [ enrollment.authorized_key ]; };
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
  services.desktopManager.plasma6.enable = true;
  services.displayManager.sddm.enable = true;
  services.displayManager.sddm.wayland.enable = true;
  services.displayManager.autoLogin.enable = false;
  services.pipewire = { enable = true; alsa.enable = true; pulse.enable = true; };
  security.rtkit.enable = true;
  # Executor1 serves authenticated preparation; model and indexer stay unloaded.
  services.aios.enable = false;
  services.aios.development = {
    enable = true;
    expectedVmUuid = guestUUID;
    expectedInstallationUuid = installationUUID;
  };
  environment.systemPackages = with pkgs; [ identity python3 git cargo rustc rustfmt clippy btrfs-progs ] ++
    [ aiosPackages.aios-cli aiosPackages.aios-core aiosPackages.aios-model aiosPackages.aios-guard ];
  environment.etc."aios/installation-uuid".text = installationUUID + "\n";
  environment.etc."aios/expected-dmi-uuid".text = guestUUID + "\n";
  environment.etc."aios/guest-role".text = "development\n";
  environment.etc."aios/management-channel".text = "ssh-development\n";
  environment.etc."aios/target-authority.json" = {
    mode = "0444";
    text = builtins.toJSON {
      schema_version = 1; os_id = "nixos"; os_version = "26.05";
      installation_uuid = installationUUID; dmi_uuid = guestUUID;
      guest_role = "development"; disk_serial = "AIOS_DEV_ROOT";
      disk_device = "vda"; root_partition = "vda2"; root_filesystem = "btrfs";
      management_channel = "ssh-development";
    };
  };
  systemd.services.aios-bootstrap-identity = {
    description = "Publish verified Horizon OS VM identity";
    wantedBy = [ "multi-user.target" ]; before = [ "sshd.service" ];
    serviceConfig.Type = "oneshot";
    script = ''
      install -d -m 0755 /run/aios
      observed=$(tr '[:upper:]' '[:lower:]' </sys/class/dmi/id/product_uuid)
      test "$observed" = ${lib.escapeShellArg guestUUID}
      printf '%s\n' "$observed" >/run/aios/dmi-uuid
      chmod 0444 /run/aios/dmi-uuid
    '';
  };
  systemd.services.sshd = { requires = [ "aios-bootstrap-identity.service" ]; after = [ "aios-bootstrap-identity.service" ]; };
}
