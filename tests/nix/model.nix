# Evaluation only: no activation or effective sandbox claim.
{ nixpkgs, aiosModel, aiosModelArtifact }:
let
  lib = nixpkgs.lib;
  base = {
    nixpkgs.hostPlatform = "x86_64-linux";
    system.stateVersion = "26.05";
    boot.loader.systemd-boot.enable = true;
    boot.loader.efi.canTouchEfiVariables = false;
    fileSystems."/" = { device = "/dev/vda2"; fsType = "ext4"; };
    fileSystems."/boot" = { device = "/dev/vda1"; fsType = "vfat"; };
    users.users.alice.isNormalUser = true;
    users.users.bob.isNormalUser = true;
  };
  evaluate = args: modules:
    let
      system = lib.nixosSystem { specialArgs = args; modules = [ base ../../nix/modules/aios ] ++ modules; };
      config = system.config;
      enabled = config.services.aios.model.enable;
    in {
      failedAssertions = map (a: a.message) (builtins.filter (a: !a.assertion) config.assertions);
      modelEnabled = enabled;
      desktopEnabled = config.services.desktopManager.plasma6.enable;
      trustedUsers = config.nix.settings.trusted-users;
      inferenceMembers = if enabled then config.users.groups.aios-inference.members else [];
      modelUser = if enabled then { inherit (config.users.users.aios-model) isSystemUser group extraGroups; } else null;
      runtime = if enabled then builtins.fromJSON config.environment.etc."aios/model-runtime.json".text else null;
      runtimeMode = if enabled then config.environment.etc."aios/model-runtime.json".mode else null;
      socketWantedBy = if enabled then config.systemd.sockets.aios-model.wantedBy else [];
      serviceWantedBy = if enabled then config.systemd.services.aios-model.wantedBy else [];
      execStart = if enabled then config.systemd.services.aios-model.serviceConfig.ExecStart else [];
      unitPackages = map toString config.systemd.packages;
      # None of these boot/login/connectivity units may depend on inference.
      bootRequires = builtins.listToAttrs (map (name: {
        inherit name;
        value = if builtins.hasAttr name config.systemd.services then config.systemd.services.${name}.requires else [];
      }) [ "sshd" "display-manager" "NetworkManager" ]);
      targetsRequire = { requires = config.systemd.targets.multi-user.requires or []; graphical = config.systemd.targets.graphical.requires or []; };
    };
  args = { inherit aiosModel aiosModelArtifact; };
  enabled = { services.aios = { users = [ "alice" "bob" ]; model = { enable = true; manifest = "${aiosModelArtifact}/lock.json"; }; }; };
in {
  disabled = evaluate args [];
  headless = evaluate args [ enabled ];
  desktop = evaluate args [ enabled { services.desktopManager.plasma6.enable = true; services.displayManager.sddm.enable = true; } ];
  zeroIdle = evaluate args [ enabled { services.aios.model.idleUnloadSeconds = 0; services.aios.model.threads = 1; } ];
  network = evaluate args [ { services.aios.model.allowNetwork = true; } ];
  unknownUser = evaluate args [ enabled { services.aios.users = lib.mkForce [ "missing" ]; } ];
  systemUser = evaluate args [ enabled { services.aios.users = lib.mkForce [ "aios-model" ]; } ];
  duplicateUser = evaluate args [ enabled { services.aios.users = lib.mkForce [ "alice" "alice" ]; } ];
  missingManifest = evaluate args [ enabled { services.aios.model.manifest = lib.mkForce null; } ];
  wrongManifest = evaluate args [ enabled { services.aios.model.manifest = lib.mkForce /etc/hosts; } ];
  missingPackages = evaluate {} [ { services.aios.model.enable = true; } ];
  extraTrust = evaluate args [ enabled { nix.settings.trusted-users = [ "root" "alice" ]; } ];
  context = evaluate args [ enabled { services.aios.model.contextTokens = 16384; } ];
  threads = evaluate args [ enabled { services.aios.model.threads = 5; } ];
  low = evaluate args [ enabled { services.aios.model.profile = "low"; } ];
  high = evaluate args [ enabled { services.aios.model.profile = "high"; } ];
}
