# A trusted template imports only canonical data frozen inside its candidate.
# Arguments are supplied by the administrator template, never a client RPC.
{ catalog, managedJSON, installationStateVersion }:
{ pkgs, lib, config, ... }:
let
  state = builtins.fromJSON managedJSON;
  exact = value: keys: builtins.isAttrs value && builtins.attrNames value == builtins.sort builtins.lessThan keys;
  ids = map (entry: entry.id) catalog.content.packages;
  entry = id: lib.findFirst (p: p.id == id) (throw "AIOS_UNKNOWN_PACKAGE") catalog.content.packages;
  profiles = [ "balanced" "power-saver" "performance" ];
  valid =
    exact state [ "schema_version" "base_template_revision" "catalog_revision" "system_packages" "services" "power_policy" ] &&
    builtins.isInt state.schema_version && state.schema_version == 1 &&
    state.base_template_revision == catalog.content.base_template_revision && state.catalog_revision == catalog.catalog_revision &&
    builtins.isList state.system_packages && builtins.length state.system_packages <= 128 &&
    state.system_packages == lib.sort builtins.lessThan (lib.unique state.system_packages) &&
    builtins.all (id: builtins.isString id && builtins.elem id ids && (entry id).capability == "desktop_application") state.system_packages &&
    exact state.services [ "postgresql" "openssh" ] &&
    exact state.services.postgresql [ "enabled" "package_id" "listen_mode" ] &&
    builtins.isBool state.services.postgresql.enabled && state.services.postgresql.package_id == "postgresql-17" && state.services.postgresql.listen_mode == "unix-only" &&
    exact state.services.openssh [ "enabled" "open_firewall" ] &&
    state.services.openssh.enabled == true && state.services.openssh.open_firewall == true &&
    exact state.power_policy [ "profile_on_ac" "profile_on_battery" ] &&
    builtins.elem state.power_policy.profile_on_ac profiles && builtins.elem state.power_policy.profile_on_battery profiles &&
    # Nix fromJSON alone is not the duplicate-key boundary. Require exactly the
    # compiler's materialized canonical JSON, rejecting duplicate/noncanonical bytes.
    managedJSON == builtins.toJSON state;
  checked = assert valid; state;
  package = id: lib.getAttrFromPath (entry id).attribute pkgs;
in {
  assertions = [
    { assertion = valid; message = "Minnerite managed data must be canonical, typed and bound to the installed catalog/template."; }
    { assertion = installationStateVersion == "26.05" && config.system.stateVersion == installationStateVersion;
      message = "Minnerite managed updates preserve the installation stateVersion baseline."; }
    { assertion = config.services.openssh.enable && config.services.openssh.openFirewall;
      message = "Minnerite cannot disable the protected management transport."; }
    { assertion = !(config.nixpkgs.config.allowUnfree or false);
      message = "Minnerite does not permit a broad allowUnfree override."; }
  ];
  environment.systemPackages = map package checked.system_packages;
  services.openssh = { enable = lib.mkDefault true; openFirewall = lib.mkDefault true; };
  services.postgresql = {
    enable = checked.services.postgresql.enabled;
    package = package "postgresql-17";
    enableTCPIP = false;
    settings.listen_addresses = lib.mkForce "";
    authentication = lib.mkForce "local all all peer";
  };
  # Runtime power application requires its own qualified fixed executor. Persist
  # the desired policy separately; template evaluation does not claim it applied.
  environment.etc."aios/power-policy.json".text = builtins.toJSON checked.power_policy;
  environment.etc."aios/managed.json".text = managedJSON;
  environment.etc."aios/catalog.json".text = builtins.toJSON catalog;
}
