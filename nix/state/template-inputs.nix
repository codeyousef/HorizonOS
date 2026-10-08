# Reviewed public source domains. Never import a worktree, HOME or mutable state.
{ root }:
let
  tree = prefix: extensions:
    let entries = builtins.readDir (root + "/${prefix}");
    in builtins.concatMap (name:
      let type = entries.${name}; path = "${prefix}/${name}";
          selected = builtins.any (suffix: builtins.match ".*${suffix}" name != null) extensions;
      in if builtins.substring 0 1 name == "." || name == "__pycache__" then []
        else if type == "directory" then tree path extensions
        else if selected then assert type == "regular"; [ path ]
        else []
    ) (builtins.attrNames entries);
  paths = builtins.sort builtins.lessThan (
    [ "flake.nix" "flake.lock" "Cargo.toml" "Cargo.lock" "dev/vm.example.json"
      "crates/aios-exec/policy/system-approval.json"
      "crates/aios-exec/policy/org.aios.executor.policy" ]
    ++ [ "crates/aios-exec/policy/org.aios.Executor1.conf" ]
    ++ tree "crates" [ "[.]rs" "[.]toml" "[.]sql" ]
    ++ tree "schemas" [ "[.]json" ]
    ++ tree "models" [ "[.]json" ]
    ++ tree "native" [ "[.]cpp" "[.]h" "[.]cmake" "CMakeLists[.]txt" ]
    ++ tree "desktop" [ "[.]cpp" "[.]hpp" "CMakeLists[.]txt" ]
    ++ tree "nix" [ "[.]nix" "[.]service" "[.]socket" "[.]path" ]
    ++ tree "tools" [ "[.]py" ]
    ++ tree "tests/unit" [ "[.]py" ]
    ++ tree "tests/nix" [ "[.]nix" ]
  );
  entry = path: {
    inherit path;
    mode = 420;
    size = builtins.stringLength (builtins.readFile (root + "/${path}"));
    sha256 = builtins.hashFile "sha256" (root + "/${path}");
  };
in { inherit paths entry; files = map entry paths; }
