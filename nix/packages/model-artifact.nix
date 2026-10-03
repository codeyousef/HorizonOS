{ lib, requireFile, runCommandNoCC }:
let
  lock = builtins.fromJSON (builtins.readFile ../../models/lock.json);
  source = builtins.fromJSON (builtins.readFile ../../models/source-lock.json);
  # No inference/code dependency and no fetcher. An administrator first imports
  # the exact reviewed conversion and original metadata with add-fixed sha256.
  required = name: sha256: requireFile {
    inherit name sha256;
    message = "Import the verified Horizon OS normal-profile artifact with nix-store --add-fixed sha256; inference never downloads model files.";
  };
  weights = required lock.artifact.filename lock.artifact.sha256;
  metadata = builtins.filter (file: file.bytes < 32 * 1024 * 1024) source.files;
in
assert lock.schema_version == 1 && lock.profile == "normal" && lock.availability == "available";
assert lock.source_lock_sha256 == builtins.hashFile "sha256" ../../models/source-lock.json;
assert source.runtime.revision == lock.runtime_revision && source.inference_backend == "cpu";
runCommandNoCC "horizon-os-model-normal-${builtins.substring 0 12 lock.artifact.sha256}" {
  meta = { license = lib.licenses.asl20; platforms = [ "x86_64-linux" ]; };
} ''
  mkdir -p "$out/source"
  install -m444 ${weights} "$out/${lock.artifact.filename}"
  test "$(stat -c %s "$out/${lock.artifact.filename}")" = ${toString lock.artifact.bytes}
  install -m444 ${../../models/lock.json} "$out/lock.json"
  install -m444 ${../../models/source-lock.json} "$out/source-lock.json"
  ${lib.concatMapStringsSep "\n" (file: ''
    install -m444 ${required file.name file.sha256} "$out/source/${file.name}"
    test "$(stat -c %s "$out/source/${file.name}")" = ${toString file.bytes}
  '') metadata}
''
