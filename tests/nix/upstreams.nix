# Read-only metadata from the locked package set, not installed-state claims.
{ pkgs, nixpkgs, imageAttributes, packageAttributes }:
let
  lib = pkgs.lib;
  paths = [ [ "linuxPackages" "kernel" ] [ "systemd" ] [ "networkmanager" ]
    [ "pipewire" ] [ "wireplumber" ] [ "bluez" ] [ "udisks2" ] [ "polkit" ]
    [ "kdePackages" "plasma-workspace" ] [ "kdePackages" "kwin" ]
    [ "kdePackages" "xdg-desktop-portal-kde" ] [ "qt6" "qtbase" ]
    [ "rustc" ] [ "cargo" ] [ "nix" ] [ "llama-cpp" ] [ "libseccomp" ] ];
  license = value: value.spdxId or (value.shortName or "unspecified");
  entry = path:
    let package = lib.attrByPath path null pkgs;
        licenses = if package == null then [] else lib.toList (package.meta.license or []);
    in { attribute = lib.concatStringsSep "." path; available = package != null;
         version = if package == null then null else package.version or null;
         licenses = map license licenses; };
in {
  schema_version = 1;
  nixpkgs_revision = nixpkgs.rev;
  nixpkgs_nar_hash = nixpkgs.narHash;
  packages = map entry paths;
  image_attributes = imageAttributes;
  project_package_attributes = packageAttributes;
  llama_source_nar_hash = pkgs.llama-cpp.src.outputHash;
  evidence_scope = "locked-source-metadata; actual image/package build and boot evidence is separate";
}
