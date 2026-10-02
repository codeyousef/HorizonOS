{ config, lib, pkgs, ... }:
let
  cfg = config.services.aios.development;
  helper = pkgs.callPackage ../../packages/dev-deploy.nix { };
  uuid = value: value != null && builtins.match "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}" value != null;
  role = (config.environment.etc."aios/guest-role" or { text = ""; }).text;
  dev = config.users.users.dev or { isNormalUser = false; extraGroups = []; };
  model = config.users.users.aios-model or { extraGroups = []; };
in {
  options.services.aios.development = {
    enable = lib.mkEnableOption "the explicit VM-only developer code deployment boundary";
    expectedVmUuid = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Canonical QEMU DMI UUID required for development authority.";
    };
    expectedInstallationUuid = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Canonical installation UUID required for development authority.";
    };
  };
  config = lib.mkIf cfg.enable {
    assertions = [
      { assertion = uuid cfg.expectedVmUuid && uuid cfg.expectedInstallationUuid;
        message = "Developer authority requires canonical DMI and installation UUIDs."; }
      { assertion = role == "development\n";
        message = "Developer authority is limited to explicitly marked development guests."; }
      { assertion = dev.isNormalUser && !(builtins.elem "wheel" dev.extraGroups);
        message = "The dedicated dev account must be normal and have no wheel authority."; }
      { assertion = !(builtins.elem "wheel" model.extraGroups);
        message = "The product model must have no developer/admin authority."; }
      { assertion = config.nix.settings.trusted-users == [ "root" ];
        message = "Only root may be a Nix trusted-user; developer code deployment is a separate authority."; }
    ];
    nix.settings.trusted-users = lib.mkDefault [ "root" ];
    environment.systemPackages = [ helper ];
    environment.etc."aios/development.json".text = builtins.toJSON {
      schema_version = 1;
      enabled = true;
      expected_vm_uuid = cfg.expectedVmUuid;
      expected_installation_uuid = cfg.expectedInstallationUuid;
      disk_serial = "AIOS_DEV_ROOT";
      management_channel = "ssh-development";
      developer_user = "dev";
    };
    security.sudo.extraRules = [ {
      users = [ "dev" ];
      runAs = "root:root";
      commands = [ {
        command = "/run/current-system/sw/bin/aios-dev-deploy --request-stdin";
        options = [ "NOPASSWD" "NOSETENV" ];
      } ];
    } ];
    systemd.tmpfiles.rules = [ "d /var/lib/aios 0755 root root -" "d /var/lib/aios/development 0700 root root -" ];
  };
}
