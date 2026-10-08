{
  description = "Horizon OS: deterministic AIOS control plane on NixOS";

  # Exact revision/NAR hash are pinned in the guest-generated flake.lock.
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      hostSource = pkgs.lib.fileset.toSource {
        root = ./.;
        fileset = pkgs.lib.fileset.unions [
          (pkgs.lib.fileset.fileFilter (file: file.hasExt "py") ./tools)
          ./dev/vm.example.json
          (pkgs.lib.fileset.fileFilter (file: file.hasExt "py") ./tests/unit)
        ];
      };
      devTools = pkgs.callPackage ./nix/packages/dev-tools.nix { src = hostSource; };
      llamaBridge = pkgs.callPackage ./nix/packages/llama-bridge.nix { };
      conversionPython = pkgs.python3.withPackages (p: [ p.numpy p.safetensors p.transformers p.sentencepiece p.protobuf p.torch ]);
      productSource = pkgs.lib.fileset.toSource {
        root = ./.;
        fileset = pkgs.lib.fileset.unions [ ./Cargo.toml ./Cargo.lock ./crates
          (pkgs.lib.fileset.fileFilter (file: file.hasExt "json") ./schemas)
          (pkgs.lib.fileset.fileFilter (file: file.hasExt "json") ./models) ];
      };
      productPackage = pname: package: program: pkgs.rustPlatform.buildRustPackage {
        inherit pname;
        version = "0.1.0";
        src = productSource;
        cargoLock.lockFile = ./Cargo.lock;
        cargoBuildFlags = [ "--package" package ];
        buildInputs = [ pkgs.systemd ];
        # Real OS integration runs separately in the verified KVM guest.
        doCheck = false;
        meta.mainProgram = program;
      };
      templateInputs = import ./nix/state/template-inputs.nix { root = ./.; };
      stateContract = import ./nix/state/catalog.nix {
        inherit pkgs;
        nixpkgsRevision = nixpkgs.rev or (builtins.fromJSON (builtins.readFile ./flake.lock)).nodes.nixpkgs.locked.rev;
        lockSha256 = builtins.hashFile "sha256" ./flake.lock;
        baseTemplateRevision = builtins.hashString "sha256" (builtins.toJSON templateInputs.files);
      };
      systemTemplate = pkgs.callPackage ./nix/packages/system-template.nix {
        root = ./.; inherit templateInputs stateContract;
      };
      state = (productPackage "aios-state" "aios-state" "aios-state-check").overrideAttrs (old: {
        AIOS_STATE_CATALOG_JSON = builtins.toJSON stateContract.catalog;
        nativeBuildInputs = (old.nativeBuildInputs or []) ++ [ pkgs.pkg-config ];
        buildInputs = (old.buildInputs or []) ++ [ pkgs.sqlite ];
      });
      cli = productPackage "aios-cli" "aios-cli" "aiosctl";
      consentUi = pkgs.callPackage ./nix/packages/consent-ui.nix { };
      consentUiTests = pkgs.callPackage ./nix/packages/consent-ui.nix { testing = true; };
      core = (productPackage "aios-core" "aios-session" "aios-sessiond").overrideAttrs (old: {
        AIOS_CONSENT_UI = "${consentUi}/bin/aios-scope-dialog";
        AIOS_WPCTL = "${pkgs.wireplumber}/bin/wpctl";
        AIOS_KREADCONFIG = "${pkgs.kdePackages.kconfig}/bin/kreadconfig6";
        AIOS_SYSTEMCTL = "${self.nixosConfigurations.aios-dev.config.systemd.package}/bin/systemctl";
        AIOS_CONSENT_NATIVE = "${consentUi}/bin/.aios-scope-dialog-wrapped";
        AIOS_USER_MANAGER = "${self.nixosConfigurations.aios-dev.config.systemd.package}/lib/systemd/systemd";
        AIOS_KWIN_WRAPPER = "${pkgs.kdePackages.kwin}/bin/.kwin_wayland_wrapper-wrapped";
        AIOS_ATSPI_LAUNCHER = "${pkgs.at-spi2-core}/libexec/.at-spi-bus-launcher-wrapped";
        AIOS_KATE_NATIVE = "${pkgs.kdePackages.kate}/bin/.kate-wrapped";
        postInstall = (old.postInstall or "") + ''
          install -Dm644 ${./nix/packages/aios-sessiond.service} "$out/share/systemd/user/aios-sessiond.service"
          substituteInPlace "$out/share/systemd/user/aios-sessiond.service" --replace-fail @EXECUTABLE@ "$out/bin/aios-sessiond"
          install -Dm644 ${./nix/packages/aios-sessiond-settings.path} "$out/share/systemd/user/aios-sessiond-settings.path"
          install -Dm644 ${./nix/packages/aios-sessiond-settings-sync.service} "$out/share/systemd/user/aios-sessiond-settings-sync.service"
          substituteInPlace "$out/share/systemd/user/aios-sessiond-settings-sync.service" --replace-fail @CP@ "${pkgs.coreutils}/bin/cp"
          install -Dm644 ${./nix/packages/aios-setting-theme.service} "$out/share/systemd/user/aios-setting-theme@.service"
          substituteInPlace "$out/share/systemd/user/aios-setting-theme@.service" --replace-fail @APPLY@ "${pkgs.kdePackages.plasma-workspace}/bin/plasma-apply-colorscheme"
          for profile in AC Battery LowBattery; do
            install -Dm644 ${./nix/packages/aios-setting-idle.service} "$out/share/systemd/user/aios-setting-idle-$profile@.service"
            substituteInPlace "$out/share/systemd/user/aios-setting-idle-$profile@.service" \
              --replace-fail @WRITE@ "${pkgs.kdePackages.kconfig}/bin/kwriteconfig6" \
              --replace-fail @PROFILE@ "$profile"
          done
          install -Dm644 ${./nix/packages/aios-ui-agent.service} "$out/share/systemd/user/aios-ui-agent.service"
          substituteInPlace "$out/share/systemd/user/aios-ui-agent.service" --replace-fail @EXECUTABLE@ "$out/bin/aios-ui-agent"
          install -Dm644 ${./nix/packages/aios-processd.service} "$out/share/systemd/user/aios-processd.service"
          substituteInPlace "$out/share/systemd/user/aios-processd.service" --replace-fail @EXECUTABLE@ "$out/bin/aios-processd"
        '';
      });
      guard = (productPackage "aios-guard" "aios-guard" "aios-guard").overrideAttrs (old: {
        nativeBuildInputs = (old.nativeBuildInputs or []) ++ [ pkgs.pkg-config ];
        buildInputs = (old.buildInputs or []) ++ [ pkgs.sqlite ];
      });
      executor = (productPackage "aios-exec" "aios-exec" "aios-execd").overrideAttrs (old: {
        AIOS_USER_MANAGER = "${self.nixosConfigurations.aios-dev.config.systemd.package}/lib/systemd/systemd";
        AIOS_NIX = "${pkgs.nix}/bin/nix";
        AIOS_NIX_STORE = "${pkgs.nix}/bin/nix-store";
        AIOS_NIXPKGS = "${pkgs.path}";
        nativeBuildInputs = (old.nativeBuildInputs or []) ++ [ pkgs.pkg-config ];
        buildInputs = (old.buildInputs or []) ++ [ pkgs.sqlite ];
        postInstall = (old.postInstall or "") + ''
          install -Dm644 ${./crates/aios-exec/policy/org.aios.executor.policy} "$out/share/polkit-1/actions/org.aios.executor.policy"
          install -Dm644 ${./crates/aios-exec/policy/system-approval.json} "$out/share/aios/system-approval.json"
          install -Dm644 ${./crates/aios-exec/policy/org.aios.Executor1.conf} "$out/share/dbus-1/system.d/org.aios.Executor1.conf"
          install -Dm644 ${./nix/packages/aios-execd.service} "$out/lib/systemd/system/aios-execd.service"
          substituteInPlace "$out/lib/systemd/system/aios-execd.service" --replace-fail @EXECUTABLE@ "$out/bin/aios-execd"
          install -Dm644 ${./nix/packages/aios-build.service} "$out/lib/systemd/system/aios-build.service"
          substituteInPlace "$out/lib/systemd/system/aios-build.service" --replace-fail @EXECUTABLE@ "$out/bin/aios-buildd"
        '';
      });
      model = (productPackage "aios-model" "aios-model" "aios-modeld").overrideAttrs (old: {
        AIOS_LLAMA_BRIDGE = "${llamaBridge}";
        AIOS_LLAMA_SOURCE = "${pkgs.llama-cpp.src}";
        postInstall = (old.postInstall or "") + ''
          install -Dm644 ${./nix/packages/aios-model.socket} "$out/share/systemd/system/aios-model.socket"
          install -Dm644 ${./nix/packages/aios-model.service} "$out/share/systemd/system/aios-model.service"
          substituteInPlace "$out/share/systemd/system/aios-model.service" --replace-fail @EXECUTABLE@ "$out/bin/aios-modeld"
          mkdir -p "$out/lib/systemd/system"
          ln -s ../../../share/systemd/system/aios-model.service "$out/lib/systemd/system/aios-model.service"
          ln -s ../../../share/systemd/system/aios-model.socket "$out/lib/systemd/system/aios-model.socket"
        '';
      });
      modelArtifact = pkgs.callPackage ./nix/packages/model-artifact.nix { };
    in {
      nixosConfigurations.aios-dev = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = { aiosModel = model; aiosModelArtifact = modelArtifact; aiosExecutor = executor; aiosTemplate = systemTemplate; aiosStateContract = stateContract; aiosState = state; aiosPackages = { aios-cli = cli; aios-core = core; aios-model = model; aios-guard = guard; }; };
        modules = [ ./nix/machines/aios-dev ];
      };
      nixosConfigurations.aios-desktop-test = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = { aiosBaseSystem = self.nixosConfigurations.aios-dev.config.system.build.toplevel; aiosModel = model; aiosModelArtifact = modelArtifact; aiosExecutor = executor; aiosTemplate = systemTemplate; aiosStateContract = stateContract; aiosState = state; aiosPackages = { aios-cli = cli; aios-core = core; aios-model = model; aios-guard = guard; }; };
        modules = [ ./nix/machines/aios-dev ./nix/machines/aios-desktop-test.nix ];
      };
      nixosConfigurations.aios-model-test = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = { aiosBaseSystem = self.nixosConfigurations.aios-dev.config.system.build.toplevel; aiosModel = model; aiosModelArtifact = modelArtifact; aiosExecutor = executor; aiosTemplate = systemTemplate; aiosStateContract = stateContract; aiosState = state; aiosPackages = { aios-cli = cli; aios-core = core; aios-model = model; aios-guard = guard; }; };
        modules = [ ./nix/machines/aios-dev ./nix/machines/aios-desktop-test.nix ./nix/machines/aios-model-test.nix ];
      };
      nixosModules.default = import ./nix/modules/aios;
      nixosModules.development = import ./nix/modules/aios;
      nixosModules.production = import ./nix/modules/aios/production.nix;
      packages.${system} = { aios-consent-ui = consentUi; aios-model-artifact = modelArtifact; aios-template = systemTemplate; aios-exec = executor; aios-state = state; aios-dev-tools = devTools; aios-guard = guard; aios-dev-deploy = pkgs.callPackage ./nix/packages/dev-deploy.nix { }; aios-cli = cli; aios-core = core; aios-model = model; aios-llama-bridge = llamaBridge; default = devTools; };
      checks.${system} = { host-unit = devTools; consent-ui = consentUiTests; };
      lib.stateContract = stateContract;
      lib.catalogPackages = builtins.listToAttrs (map (entry: {
        name = entry.id;
        value = pkgs.lib.getAttrFromPath entry.attribute pkgs;
      }) stateContract.catalog.content.packages);
      lib.managedState = import ./tests/nix/managed.nix { inherit nixpkgs stateContract; };
      lib.developmentBoundary = import ./tests/nix/development.nix { inherit nixpkgs; aiosCore = core; };
      lib.modelModule = import ./tests/nix/model.nix { inherit nixpkgs; aiosModel = model; aiosModelArtifact = modelArtifact; };
      lib.modelOptionsDocumentation = (pkgs.nixosOptionsDoc {
        options.services.aios = (nixpkgs.lib.nixosSystem { inherit system; modules = [ ./nix/modules/aios ]; }).options.services.aios;
        warningsAreErrors = true;
      }).optionsCommonMark;
      lib.upstreamCompatibility = import ./tests/nix/upstreams.nix {
        inherit pkgs nixpkgs;
        imageAttributes = builtins.attrNames self.nixosConfigurations;
        packageAttributes = builtins.attrNames self.packages.${system};
      };
      devShells.${system} = {
      lock-resolution = pkgs.mkShell { packages = [ pkgs.cargo pkgs.rustc ]; };
      default = pkgs.mkShell {
        packages = with pkgs; [ python3 git openssh cargo rustc rustfmt clippy pkg-config sqlite systemd ];
        AIOS_CONSENT_UI = "${consentUi}/bin/aios-scope-dialog";
        AIOS_CONSENT_NATIVE = "${consentUi}/bin/.aios-scope-dialog-wrapped";
        AIOS_USER_MANAGER = "${self.nixosConfigurations.aios-dev.config.systemd.package}/lib/systemd/systemd";
        AIOS_KWIN_WRAPPER = "${pkgs.kdePackages.kwin}/bin/.kwin_wayland_wrapper-wrapped";
        AIOS_ATSPI_LAUNCHER = "${pkgs.at-spi2-core}/libexec/.at-spi-bus-launcher-wrapped";
        AIOS_KATE_NATIVE = "${pkgs.kdePackages.kate}/bin/.kate-wrapped";
        AIOS_KATE_LAUNCH = "${pkgs.kdePackages.kate}/bin/kate";
        AIOS_LLAMA_BRIDGE = "${llamaBridge}";
        AIOS_LLAMA_SOURCE = "${pkgs.llama-cpp.src}";
      };
      model-conversion = pkgs.mkShell {
        packages = [ conversionPython llamaBridge ];
        AIOS_LLAMA_SOURCE = "${pkgs.llama-cpp.src}";
        AIOS_LLAMA_BRIDGE = "${llamaBridge}";
        AIOS_PROFILE_SECCOMP = "${pkgs.libseccomp.lib}/lib/libseccomp.so.2";
        PYTHONPATH = "${pkgs.llama-cpp.src}/gguf-py";
        HF_HUB_OFFLINE = "1";
      };
      };
    };
}
