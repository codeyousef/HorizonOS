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
      cli = productPackage "aios-cli" "aios-cli" "aiosctl";
      core = (productPackage "aios-core" "aios-session" "aios-sessiond").overrideAttrs (old: {
        postInstall = (old.postInstall or "") + ''
          install -Dm644 ${./nix/packages/aios-sessiond.service} "$out/share/systemd/user/aios-sessiond.service"
          substituteInPlace "$out/share/systemd/user/aios-sessiond.service" --replace-fail @EXECUTABLE@ "$out/bin/aios-sessiond"
        '';
      });
      model = (productPackage "aios-model" "aios-model" "aios-model-probe").overrideAttrs (old: {
        AIOS_LLAMA_BRIDGE = "${llamaBridge}";
      });
    in {
      nixosModules.default = import ./nix/modules/aios;
      nixosModules.development = import ./nix/modules/aios/development.nix;
      packages.${system} = { aios-dev-tools = devTools; aios-cli = cli; aios-core = core; aios-model = model; aios-llama-bridge = llamaBridge; default = devTools; };
      checks.${system}.host-unit = devTools;
      devShells.${system} = {
      lock-resolution = pkgs.mkShell { packages = [ pkgs.cargo pkgs.rustc ]; };
      default = pkgs.mkShell {
        packages = with pkgs; [ python3 git openssh cargo rustc rustfmt clippy ];
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
