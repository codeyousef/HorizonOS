# Pure module evaluation fixtures. These do not activate any guest configuration.
{ nixpkgs }:
let
  lib = nixpkgs.lib;
  uuid = "12345678-1234-4234-8234-123456789abc";
  base = {
    nixpkgs.hostPlatform = "x86_64-linux";
    system.stateVersion = "26.05";
    boot.loader.systemd-boot.enable = true;
    boot.loader.efi.canTouchEfiVariables = false;
    fileSystems."/" = { device = "/dev/vda2"; fsType = "ext4"; };
    fileSystems."/boot" = { device = "/dev/vda1"; fsType = "vfat"; };
  };
  enabled = {
    users.users.dev.isNormalUser = true;
    environment.etc."aios/guest-role".text = "development\n";
    services.aios.development = { enable = true; expectedVmUuid = uuid; expectedInstallationUuid = uuid; };
  };
  evaluate = modules:
    let config = (lib.nixosSystem { modules = [ base ] ++ modules; }).config;
    in {
      failedAssertions = map (entry: entry.message) (builtins.filter (entry: !entry.assertion) config.assertions);
      helperPresent = builtins.any (package: (package.pname or "") == "aios-dev-deploy") config.environment.systemPackages;
      trustedUsers = config.nix.settings.trusted-users;
      developerRules = builtins.filter (rule: builtins.elem "dev" rule.users) config.security.sudo.extraRules;
      developmentEnabled = config.services.aios.development.enable;
    };
  development = ../../nix/modules/aios/default.nix;
  production = ../../nix/modules/aios/production.nix;
in {
  development = evaluate [ development enabled ];
  disabled = evaluate [ development ];
  wrongRole = evaluate [ development (enabled // { environment.etc."aios/guest-role".text = "production\n"; }) ];
  missingUuid = evaluate [ development enabled { services.aios.development.expectedVmUuid = lib.mkForce null; } ];
  extraNixTrust = evaluate [ development enabled { nix.settings.trusted-users = [ "root" "dev" ]; } ];
  wheelDeveloper = evaluate [ development enabled { users.users.dev.extraGroups = [ "wheel" ]; } ];
  wheelModel = evaluate [ development enabled { users.users.aios-model = { isSystemUser = true; group = "aios-model"; extraGroups = [ "wheel" ]; }; users.groups.aios-model = {}; } ];
  production = evaluate [ production ];
  productionHelper = evaluate [ production enabled ];
  productionSudoHelper = evaluate [ production { security.sudo.extraRules = [ { users = [ "dev" ]; commands = [ { command = "/run/current-system/sw/bin/aios-dev-deploy --request-stdin"; options = [ "NOPASSWD" ]; } ]; } ]; } ];
  productionPasswordlessAll = evaluate [ production { security.sudo.extraRules = [ { users = [ "dev" ]; commands = [ { command = "ALL"; options = [ "NOPASSWD" ]; } ]; } ]; } ];
  productionPasswordlessWheel = evaluate [ production { security.sudo.wheelNeedsPassword = false; } ];
  productionDesktopAutologin = evaluate [ production { services.displayManager.autoLogin = { enable = true; user = "alice"; }; } ];
  productionConsoleAutologin = evaluate [ production { services.getty.autologinUser = "alice"; } ];
  productionDevAccount = evaluate [ production { users.users.dev.isNormalUser = true; } ];
  productionTesterAccount = evaluate [ production { users.users.tester.isNormalUser = true; } ];
}
