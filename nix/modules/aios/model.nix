# Administrator-owned inference configuration. Requests cannot change these
# options, import models, choose executables or obtain task/tool authority.
{ config, lib, ... }@moduleArgs:
let
  cfg = config.services.aios;
  aiosModel = moduleArgs.aiosModel or null;
  aiosModelArtifact = moduleArgs.aiosModelArtifact or null;
  manifest = if aiosModelArtifact == null then null else "${aiosModelArtifact}/lock.json";
in {
  options.services.aios = {
    users = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [];
      description = "Existing normal users explicitly allowed to connect to local inference; this grants no file or tool access.";
    };
    model = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = cfg.enable;
        description = "Enable the independently socket-activated local CPU model.";
      };
      profile = lib.mkOption { type = lib.types.enum [ "normal" "low" "high" ]; default = "normal"; description = "Reviewed model profile; only normal currently has a verified artifact."; };
      manifest = lib.mkOption { type = lib.types.nullOr lib.types.path; default = null; description = "Manifest in the reviewed immutable model artifact; required when inference is enabled."; };
      contextTokens = lib.mkOption { type = lib.types.ints.positive; default = 8192; description = "Native context allocation; the qualified normal profile requires 8192 tokens."; };
      threads = lib.mkOption { type = lib.types.nullOr lib.types.ints.positive; default = null; description = "CPU threads, at most four and no more than available cores. Null reserves a core where possible and chooses at most four."; };
      idleUnloadSeconds = lib.mkOption { type = lib.types.ints.unsigned; default = 600; description = "Idle time before unloading weights. Zero disables automatic unloading; explicit unload and all quotas remain enforced."; };
      allowNetwork = lib.mkOption { type = lib.types.bool; default = false; description = "Network access is prohibited in V1 and true is rejected."; };
    };
  };
  config = lib.mkMerge [ {
    assertions = [
      { assertion = !cfg.model.allowNetwork; message = "Minnerite V1 model.allowNetwork must be false."; }
      { assertion = builtins.all (name: builtins.hasAttr name config.users.users && config.users.users.${name}.isNormalUser) cfg.users;
        message = "Minnerite inference users must be existing normal users."; }
      { assertion = builtins.length cfg.users == builtins.length (lib.unique cfg.users); message = "Minnerite inference users must be unique."; }
      { assertion = cfg.model.threads == null || cfg.model.threads <= 4; message = "Minnerite model threads are bounded to four."; }
    ];
  } (lib.mkIf cfg.model.enable {
    assertions = [
      { assertion = aiosModel != null && aiosModelArtifact != null; message = "Enabled inference requires the reviewed code and immutable artifact packages."; }
      { assertion = cfg.model.profile == "normal" && cfg.model.contextTokens == 8192; message = "Only the verified normal 8192-token model profile can be enabled."; }
      { assertion = cfg.model.manifest != null && toString cfg.model.manifest == manifest; message = "Enabled inference requires the reviewed artifact's exact manifest."; }
      { assertion = config.nix.settings.trusted-users == [ "root" ]; message = "Inference deployment requires root-only Nix trust."; }
    ];
    users.users.aios-model = { isSystemUser = true; group = "aios-model"; };
    users.groups.aios-model = {};
    users.groups.aios-inference.members = cfg.users;
    environment.etc."aios/model-runtime.json" = {
      mode = "symlink";
      text = builtins.toJSON { schema_version = 1; profile = cfg.model.profile; context_tokens = cfg.model.contextTokens;
        threads = cfg.model.threads; idle_unload_seconds = cfg.model.idleUnloadSeconds; };
    };
    systemd.tmpfiles.rules = [ "d /run/aios 0755 root root -" "d /var/lib/aios 0755 root root -" "d /var/lib/aios/models 0755 root root -" ];
    systemd.packages = lib.optional (aiosModel != null) aiosModel;
    systemd.sockets.aios-model = { wantedBy = [ "sockets.target" ]; overrideStrategy = "asDropin"; };
    systemd.services.aios-model = {
      overrideStrategy = "asDropin";
      serviceConfig.ExecStart = [ "" "${if aiosModel == null then "/unavailable" else aiosModel}/bin/aios-modeld --model-directory ${if aiosModelArtifact == null then "/unavailable" else aiosModelArtifact} --runtime-config /etc/aios/model-runtime.json" ];
    };
  }) ];
}
