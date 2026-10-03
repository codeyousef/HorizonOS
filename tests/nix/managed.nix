# Pure NixOS evaluations only. No installation, root registration or activation.
{ nixpkgs, stateContract }:
let
  lib = nixpkgs.lib;
  base = { nixpkgs.hostPlatform = "x86_64-linux"; system.stateVersion = "26.05"; boot.loader.systemd-boot.enable = true; boot.loader.efi.canTouchEfiVariables = false; fileSystems."/" = { device = "/dev/vda2"; fsType = "ext4"; }; fileSystems."/boot" = { device = "/dev/vda1"; fsType = "vfat"; }; };
  evaluate = json: overrides:
    let config = (lib.nixosSystem {
      modules = [ base (import ../../nix/state/managed.nix {
        catalog = stateContract.catalog; managedJSON = json; installationStateVersion = "26.05";
      }) overrides ];
    }).config;
    in {
      failedAssertions = map (a: a.message) (builtins.filter (a: !a.assertion) config.assertions);
      packages = map (p: p.pname or p.name) (builtins.filter (p: builtins.elem (p.pname or "") [ "kate" "kcalc" "blender" ]) config.environment.systemPackages);
      postgresql = { enabled = config.services.postgresql.enable; version = config.services.postgresql.package.version;
        listen = config.services.postgresql.settings.listen_addresses; tcp = config.services.postgresql.enableTCPIP;
        authentication = config.services.postgresql.authentication; };
      openssh = { enabled = config.services.openssh.enable; open_firewall = config.services.openssh.openFirewall; };
      inherit (config.system) stateVersion;
      managedJSON = config.environment.etc."aios/managed.json".text;
      powerJSON = config.environment.etc."aios/power-policy.json".text;
    };
  defaults = stateContract.defaults;
  installed = defaults // { system_packages = [ "kate" "kcalc" ]; };
  postgres = defaults // { services = defaults.services // { postgresql = defaults.services.postgresql // { enabled = true; }; }; };
  strict = json: (builtins.tryEval (builtins.deepSeq (evaluate json {}) true)).success;
in {
  defaults = evaluate (builtins.toJSON defaults) {};
  packages = evaluate (builtins.toJSON installed) {};
  postgresql = evaluate (builtins.toJSON postgres) {};
  changedStateVersion = evaluate (builtins.toJSON defaults) { system.stateVersion = lib.mkForce "26.11"; };
  closedFirewall = evaluate (builtins.toJSON defaults) { services.openssh.openFirewall = lib.mkForce false; };
  broadUnfree = evaluate (builtins.toJSON defaults) { nixpkgs.config.allowUnfree = true; };
  denials = {
    arbitraryNix = strict (builtins.toJSON (defaults // { imports = [ "/var/lib/aios/mutable.nix" ]; }));
    arbitraryPackage = strict (builtins.toJSON (defaults // { system_packages = [ "pkgs.runCommand" ]; }));
    disabledSSH = strict (builtins.toJSON (defaults // { services = defaults.services // { openssh = { enabled = false; open_firewall = true; }; }; }));
    exposedPostgres = strict (builtins.toJSON (defaults // { services = defaults.services // { postgresql = defaults.services.postgresql // { listen_mode = "tcp"; }; }; }));
    changedCatalog = strict (builtins.toJSON (defaults // { catalog_revision = lib.concatStrings (lib.replicate 64 "0"); }));
    duplicateKeys = strict ("{\"schema_version\":1," + builtins.substring 1 (builtins.stringLength (builtins.toJSON defaults)) (builtins.toJSON defaults));
    noncanonical = strict (builtins.toJSON defaults + "\n");
  };
}
