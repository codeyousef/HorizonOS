# Administrator-reviewed mapping. Model/client input never selects attributes.
{ pkgs, nixpkgsRevision, lockSha256, baseTemplateRevision }:
let
  lib = pkgs.lib;
  reviewed = [
    { id = "blender"; attribute = [ "blender" ]; display_name = "Blender"; binaries = [ "blender" ]; desktop_ids = [ "blender.desktop" ]; capability = "desktop_application"; }
    { id = "kate"; attribute = [ "kdePackages" "kate" ]; display_name = "Kate"; binaries = [ "kate" ]; desktop_ids = [ "org.kde.kate.desktop" ]; capability = "desktop_application"; }
    { id = "kcalc"; attribute = [ "kdePackages" "kcalc" ]; display_name = "KCalc"; binaries = [ "kcalc" ]; desktop_ids = [ "org.kde.kcalc.desktop" ]; capability = "desktop_application"; }
    { id = "postgresql-17"; attribute = [ "postgresql_17" ]; display_name = "PostgreSQL 17"; binaries = [ "pg_isready" ]; desktop_ids = []; capability = "postgresql17"; }
  ];
  reviewedOptions = [
    { id = "power_policy.profile_on_ac"; value_kind = "power_profile"; }
    { id = "power_policy.profile_on_battery"; value_kind = "power_profile"; }
    { id = "services.openssh.enabled"; value_kind = "boolean"; }
    { id = "services.openssh.open_firewall"; value_kind = "boolean"; }
    { id = "services.postgresql.enabled"; value_kind = "boolean"; }
    { id = "services.postgresql.listen_mode"; value_kind = "postgresql_listen_mode"; }
    { id = "services.postgresql.package_id"; value_kind = "package_id"; }
  ];
  entry = spec:
    let
      package = lib.getAttrFromPath spec.attribute pkgs;
      licenses = lib.toList package.meta.license;
      payload = spec // {
        version = package.version;
        platform = "x86_64-linux";
        licenses = map (license: license.spdxId or license.shortName) licenses;
        unfree = builtins.any (license: !(license.free or true)) licenses;
      };
    in assert lib.meta.availableOn pkgs.stdenv.hostPlatform package;
      assert !payload.unfree; # Initial catalog is entirely free; no broad allowUnfree.
      payload // { metadata_revision = builtins.hashString "sha256" (builtins.toJSON payload); };
  optionEntry = spec: spec // {
    metadata_revision = builtins.hashString "sha256" (builtins.toJSON spec);
  };
  content = {
    schema_version = 1;
    base_template_revision = baseTemplateRevision;
    lock_sha256 = lockSha256;
    nixpkgs_revision = nixpkgsRevision;
    installation_state_version = "26.05";
    platform = "x86_64-linux";
    packages = map entry reviewed;
    options = map optionEntry reviewedOptions;
  };
  catalog = { inherit content; catalog_revision = builtins.hashString "sha256" (builtins.toJSON content); };
in {
  inherit catalog;
  defaults = {
    schema_version = 1;
    base_template_revision = baseTemplateRevision;
    catalog_revision = catalog.catalog_revision;
    system_packages = [];
    services = {
      postgresql = { enabled = false; package_id = "postgresql-17"; listen_mode = "unix-only"; };
      openssh = { enabled = true; open_firewall = true; };
    };
    power_policy = { profile_on_ac = "balanced"; profile_on_battery = "power-saver"; };
  };
}
