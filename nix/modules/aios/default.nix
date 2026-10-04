{ config, lib, ... }:
{
  imports = [ ./development.nix ./model.nix ];
  options.services.aios.enable = lib.mkEnableOption "the Horizon OS control plane";
  config.assertions = [ {
    assertion = !config.services.aios.enable;
    message = "Horizon OS services are not fully implemented; this scaffold cannot enable an operational control plane.";
  } ];
}
