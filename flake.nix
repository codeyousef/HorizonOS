{
  description = "Horizon OS: deterministic AIOS control plane on NixOS";

  # Initial selection; the verified guest workflow generates and validates the lock.
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
    in {
      nixosModules.default = import ./nix/modules/aios;
      nixosModules.development = import ./nix/modules/aios/development.nix;
      packages.${system} = { aios-dev-tools = devTools; default = devTools; };
      checks.${system}.host-unit = devTools;
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [ python3 git openssh cargo rustc rustfmt clippy ];
      };
    };
}
