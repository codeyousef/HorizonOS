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
          (pkgs.lib.fileset.fileFilter (file: file.hasExt "json") ./models) ];
      };
      productPackage = pname: package: program: pkgs.rustPlatform.buildRustPackage {
        inherit pname;
        version = "0.1.0";
        src = productSource;
        cargoLock.lockFile = ./Cargo.lock;
        cargoBuildFlags = [ "--package" package ];
        # Real OS integration runs separately in the verified KVM guest.
        doCheck = false;
        meta.mainProgram = program;
      };
      templateInputs = import ./nix/state/template-inputs.nix { root = ./.; };
      stateContract = import ./nix/state/catalog.nix {
        inherit pkgs;
        nixpkgsRevision = nixpkgs.rev;
        lockSha256 = builtins.hashFile "sha256" ./flake.lock;
        baseTemplateRevision = builtins.hashString "sha256" (builtins.toJSON templateInputs.files);
      };
      systemTemplate = pkgs.callPackage ./nix/packages/system-template.nix {
        root = ./.; inherit templateInputs stateContract;
      };
      state = (productPackage "aios-state" "aios-state" "aios-state-check").overrideAttrs (_: {
        AIOS_STATE_CATALOG_JSON = builtins.toJSON stateContract.catalog;
      });
      cli = productPackage "aios-cli" "aios-cli" "aiosctl";
      core = (productPackage "aios-core" "aios-session" "aios-sessiond").overrideAttrs (old: {
        postInstall = (old.postInstall or "") + ''
          install -Dm644 ${./nix/packages/aios-sessiond.service} "$out/share/systemd/user/aios-sessiond.service"
          substituteInPlace "$out/share/systemd/user/aios-sessiond.service" --replace-fail @EXECUTABLE@ "$out/bin/aios-sessiond"
        '';
      });
      guard = (productPackage "aios-guard" "aios-guard" "aios-guard").overrideAttrs (old: {
        nativeBuildInputs = (old.nativeBuildInputs or []) ++ [ pkgs.pkg-config ];
        buildInputs = (old.buildInputs or []) ++ [ pkgs.sqlite ];
      });
      executor = (productPackage "aios-exec" "aios-exec" "aios-execd").overrideAttrs (old: {
        nativeBuildInputs = (old.nativeBuildInputs or []) ++ [ pkgs.pkg-config ];
        buildInputs = (old.buildInputs or []) ++ [ pkgs.sqlite ];
      });
      model = (productPackage "aios-model" "aios-model" "aios-modeld").overrideAttrs (old: {
        AIOS_LLAMA_BRIDGE = "${llamaBridge}";
        postInstall = (old.postInstall or "") + ''
          install -Dm644 ${./nix/packages/aios-model.socket} "$out/share/systemd/system/aios-model.socket"
          install -Dm644 ${./nix/packages/aios-model.service} "$out/share/systemd/system/aios-model.service"
          substituteInPlace "$out/share/systemd/system/aios-model.service" --replace-fail @EXECUTABLE@ "$out/bin/aios-modeld"
        '';
      });
    in {
      nixosConfigurations.aios-dev = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = { aiosTemplate = systemTemplate; aiosStateContract = stateContract; aiosPackages = { aios-cli = cli; aios-core = core; aios-model = model; aios-guard = guard; }; };
        modules = [ ./nix/machines/aios-dev ];
      };
      nixosModules.default = import ./nix/modules/aios;
      nixosModules.development = import ./nix/modules/aios;
      nixosModules.production = import ./nix/modules/aios/production.nix;
      packages.${system} = { aios-template = systemTemplate; aios-exec = executor; aios-state = state; aios-dev-tools = devTools; aios-guard = guard; aios-dev-deploy = pkgs.callPackage ./nix/packages/dev-deploy.nix { }; aios-cli = cli; aios-core = core; aios-model = model; aios-llama-bridge = llamaBridge; default = devTools; };
      checks.${system}.host-unit = devTools;
      lib.stateContract = stateContract;
      lib.managedState = import ./tests/nix/managed.nix { inherit nixpkgs stateContract; };
      lib.developmentBoundary = import ./tests/nix/development.nix { inherit nixpkgs; };
      devShells.${system} = {
      lock-resolution = pkgs.mkShell { packages = [ pkgs.cargo pkgs.rustc ]; };
      default = pkgs.mkShell {
        packages = with pkgs; [ python3 git openssh cargo rustc rustfmt clippy pkg-config sqlite ];
        AIOS_LLAMA_BRIDGE = "${llamaBridge}";
      };
      model-conversion = pkgs.mkShell {
        packages = [ conversionPython llamaBridge ];
        AIOS_LLAMA_SOURCE = "${pkgs.llama-cpp.src}";
        AIOS_LLAMA_BRIDGE = "${llamaBridge}";
        PYTHONPATH = "${pkgs.llama-cpp.src}/gguf-py";
        HF_HUB_OFFLINE = "1";
      };
      };
    };
}
