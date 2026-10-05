# Install the reviewed user broker independently of inference availability.
{ config, lib, ... }@moduleArgs:
let
  cfg = config.services.aios;
  core = moduleArgs.aiosCore or ((moduleArgs.aiosPackages or {}).aios-core or null);
in {
  options.services.aios.session.enable = lib.mkOption {
    type = lib.types.bool;
    default = cfg.enable;
    description = "Start the packaged authenticated user broker for logged-in users, independently of CPU inference. This grants no model, file, tool or privileged execution authority.";
  };
  config = lib.mkIf cfg.session.enable {
    assertions = [{
      assertion = core != null;
      message = "Enabled Horizon OS user broker requires the reviewed aiosCore package.";
    }];
    environment.systemPackages = lib.optional (core != null) core;
    systemd.packages = lib.optional (core != null) core;
    systemd.user.services.aios-sessiond = {
      wantedBy = [ "default.target" ];
      overrideStrategy = "asDropin";
    };
    systemd.user.services.aios-processd = {
      wantedBy = [ "default.target" ];
      overrideStrategy = "asDropin";
    };
  };
}
