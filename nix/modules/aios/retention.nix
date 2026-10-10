{ config, ... }:
let
  cfg = config.services.aios.transactions;
in {
  environment.etc."aios/transaction-policy.json" = {
    mode = "0444";
    text = builtins.toJSON {
      schema_version = 1;
      keep_known_good_generations = cfg.keepKnownGoodGenerations;
    };
  };
  systemd.tmpfiles.rules = [
    "d /nix/var/nix/gcroots/aios-known-good 0700 root root -"
  ];
}
