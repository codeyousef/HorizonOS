{ config, lib, ... }:
{
  options.services.aios.enable = lib.mkEnableOption "the AIOS control plane";
  config.assertions = [ {
    assertion = !config.services.aios.enable;
    message = "AIOS services are not implemented; this scaffold cannot enable an operational control plane.";
  } ];
}
