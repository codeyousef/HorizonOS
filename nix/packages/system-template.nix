{ pkgs, root, templateInputs, stateContract }:
let
  enrollment = "nix/machines/aios-dev/enrollment.json";
  paths = builtins.sort builtins.lessThan (templateInputs.paths ++ [ enrollment ]);
  catalogJSON = builtins.toJSON stateContract.catalog;
  catalogEntry = {
    path = "catalog.json"; mode = 420;
    size = builtins.stringLength catalogJSON;
    sha256 = builtins.hashString "sha256" catalogJSON;
  };
  files = builtins.sort (a: b: a.path < b.path) ((map templateInputs.entry paths) ++ [ catalogEntry ]);
  manifest = { schema_version = 1; inherit files; };
  manifestJSON = builtins.toJSON manifest;
  source = pkgs.lib.fileset.toSource {
    inherit root;
    fileset = pkgs.lib.fileset.unions (map (path: root + "/${path}") paths);
  };
  manifestFile = pkgs.writeText "aios-template-manifest.json" manifestJSON;
  catalogFile = pkgs.writeText "aios-template-catalog.json" catalogJSON;
in assert builtins.pathExists (root + "/${enrollment}");
  # Reserve one entry and the compiler's maximum managed input for candidates.
  assert builtins.length files + 1 <= 4096;
  assert builtins.stringLength manifestJSON <= 1024 * 1024;
  assert builtins.all (file:
    builtins.stringLength file.path <= 512 &&
    builtins.length (pkgs.lib.splitString "/" file.path) <= 16 &&
    builtins.match "[A-Za-z0-9_.-]+(/[A-Za-z0-9_.-]+)*" file.path != null &&
    file.size <= 16 * 1024 * 1024
  ) files;
  assert builtins.foldl' (total: file: total + file.size) 0 files + 65536 <= 64 * 1024 * 1024;
  pkgs.runCommand "aios-system-template-0.1.0" {
    passthru = {
      inherit manifest;
      manifestSha256 = builtins.hashString "sha256" manifestJSON;
      authority = {
        schema_version = 1;
        manifest_sha256 = builtins.hashString "sha256" manifestJSON;
        base_template_revision = stateContract.catalog.content.base_template_revision;
        catalog_revision = stateContract.catalog.catalog_revision;
        lock_sha256 = stateContract.catalog.content.lock_sha256;
      };
    };
  } ''
    mkdir "$out"
    cp -r ${source}/. "$out/"
    chmod -R u+w "$out"
    cp ${catalogFile} "$out/catalog.json"
    cp ${manifestFile} "$out/template.json"
    find "$out" -type f -exec chmod 0444 {} +
    find "$out" -type d -exec chmod 0555 {} +
  ''
