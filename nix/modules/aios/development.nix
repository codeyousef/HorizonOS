{ config, lib, ... }:
{
  imports = [ ./default.nix ];
  options.services.aios.development.enable = lib.mkEnableOption "the explicit guest development deployment route";
  config.assertions = [ {
    assertion = !config.services.aios.development.enable;
    message = "The verified guest deployment broker is not implemented; no development privilege is granted.";
  } ];
}
