{ lib, stdenvNoCC, python3, makeWrapper, git, src }:
stdenvNoCC.mkDerivation {
  pname = "aios-dev-tools";
  version = "0.1.0-dev";
  inherit src;
  nativeBuildInputs = [ python3 makeWrapper ];
  nativeCheckInputs = [ git ];
  dontBuild = true;
  doCheck = true;
  checkPhase = ''
    runHook preCheck
    python3 -m unittest discover -s tests/unit -v
    runHook postCheck
  '';
  installPhase = ''
    runHook preInstall
    mkdir -p $out/share/aios-dev-tools $out/bin
    cp -r tools dev $out/share/aios-dev-tools/
    makeWrapper ${python3}/bin/python3 $out/bin/devctl \
      --add-flags $out/share/aios-dev-tools/tools/devctl.py
    runHook postInstall
  '';
  meta = {
    description = "Verified AIOS guest development tooling";
    mainProgram = "devctl";
    platforms = lib.platforms.linux;
  };
}
