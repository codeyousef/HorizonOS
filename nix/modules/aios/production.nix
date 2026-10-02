{ config, lib, ... }:
{
  imports = [ ./default.nix ];
  environment.etc."aios/guest-role".text = lib.mkDefault "production\n";
  assertions = [
    { assertion = !config.services.aios.development.enable;
      message = "Production excludes VM-only developer authority."; }
    { assertion = builtins.all (package: (package.pname or "") != "aios-dev-deploy") config.environment.systemPackages;
      message = "Production excludes the development deployment helper package."; }
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
  ];
  nix.settings.trusted-users = lib.mkDefault [ "root" ];
}
