{ config, lib, ... }:
let
  cfg = config.services.aios;
in {
  imports = [ ./development.nix ./model.nix ./session.nix ./graph.nix ./retention.nix ];
  options.services.aios = {
    enable = lib.mkEnableOption "the Minnerite control plane";
    desktop = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable graphical Minnerite clients; headless configurations leave this false.";
      };
      visualControl.enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable consented portal visual control only after its compatibility and safety gate passes.";
      };
    };
    index.enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable the per-user index service; indexing roots still require user consent.";
    };
    proactive.enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Enable opted-in proactive diagnosis; mutations remain forbidden.";
    };
    automation.enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable deterministic approved automation rules; this grants no rule authority.";
    };
    recovery = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable installed recovery integration; Minnerite image configurations opt in explicitly.";
      };
      initrdDiagnostics.enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable bounded deterministic initrd diagnostics without inference.";
      };
    };
    observability.kernelProbes.enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Enable only reviewed bounded kernel probes; disabled by default.";
    };
    transactions = {
      guardTimeoutSeconds = lib.mkOption {
        type = lib.types.ints.between 60 3600;
        default = 180;
        description = "Deadline for independent guarded activation verification.";
      };
      keepKnownGoodGenerations = lib.mkOption {
        type = lib.types.ints.between 3 100;
        default = 3;
        description = "Distinct recent known-good generations retained outside active transaction roots.";
      };
    };
  };
  config.assertions = [
    {
      assertion = !cfg.desktop.visualControl.enable || cfg.desktop.enable;
      message = "Minnerite visual control requires desktop.enable.";
    }
    {
      assertion = !cfg.recovery.initrdDiagnostics.enable || cfg.recovery.enable;
      message = "Minnerite initrd diagnostics require recovery.enable.";
    }
  ];
}
