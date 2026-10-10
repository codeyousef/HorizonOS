{ config, lib, ... }:
{
  imports = [ ./default.nix ];
  environment.etc."aios/guest-role".text = lib.mkDefault "production\n";
  assertions = [
    { assertion = !config.services.aios.development.enable;
      message = "Production excludes VM-only developer authority."; }
    { assertion = builtins.all (package: (package.pname or "") != "aios-dev-deploy") config.environment.systemPackages;
      message = "Production excludes the development deployment helper package."; }
    { assertion = builtins.all (package: (package.pname or "") != "aios-dev-deploy") config.systemd.packages;
      message = "Production excludes the developer activation guard unit package."; }
    { assertion = config.nix.settings.trusted-users == [ "root" ];
      message = "Only root may be a Nix trusted-user in production."; }
    { assertion = builtins.all (rule:
        builtins.all (command:
          let text = if builtins.isString command then command else command.command;
              options = if builtins.isString command then [] else command.options;
          in !(lib.hasInfix "aios-dev-deploy" text) &&
            !(text == "ALL" && builtins.elem "NOPASSWD" options && rule.users != [ "root" ])
        ) rule.commands
      ) config.security.sudo.extraRules;
      message = "Production excludes developer helper rules and unrestricted passwordless sudo."; }
    { assertion = config.security.sudo.wheelNeedsPassword;
      message = "Production excludes unrestricted passwordless wheel sudo."; }
    { assertion = !config.services.displayManager.autoLogin.enable && config.services.getty.autologinUser == null;
      message = "Production excludes graphical and console acceptance autologin."; }
    { assertion = !(config.users.users ? dev) && !(config.users.users ? tester);
      message = "Production excludes reserved development and tester accounts."; }
    { assertion = builtins.all (name: !(builtins.hasAttr name config.environment.etc)) [
        "aios/desktop-test-profile" "aios/graph-test-profile"
        "aios/model-test-profile" "aios/service-fixture-profile"
      ];
      message = "Production excludes disposable test scenario profiles."; }
    { assertion = builtins.all (name: !(builtins.hasAttr name config.systemd.services)) [
        "aios-storage-full-fixture" "aios-removable-device-fixture"
        "aios-service-lifecycle-test" "aios-builder-qualification"
        "aios-graph-denied-probe" "aios-graph-acceptance"
        "aios-service-failure-fixture" "aios-service-restart-fixture"
        "aios-model-acceptance"
      ];
      message = "Production excludes fixed acceptance fixture services."; }
    { assertion = builtins.all (name: !(builtins.hasAttr name config.systemd.paths)) [
        "aios-service-lifecycle-test" "aios-builder-qualification"
      ] && !(builtins.hasAttr "aios-removable-device-fixture" config.systemd.timers);
      message = "Production excludes fixed acceptance fixture activation units."; }
  ];
  nix.settings.trusted-users = lib.mkDefault [ "root" ];
}
