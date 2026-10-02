{ lib, stdenvNoCC, python3, makeWrapper }:
stdenvNoCC.mkDerivation {
  pname = "aios-dev-deploy";
  version = "0.1.0";
  dontUnpack = true;
  dontBuild = true;
  nativeBuildInputs = [ makeWrapper ];
  installPhase = ''
    mkdir -p $out/lib/aios-dev-deploy $out/bin
    cp ${../../tools/guest/dev_deploy.py} $out/lib/aios-dev-deploy/deploy.py
    cp ${../../tools/guest/snapshot.py} $out/lib/aios-dev-deploy/snapshot.py
    makeWrapper ${python3}/bin/python3 $out/bin/aios-dev-deploy \
      --add-flags "-I $out/lib/aios-dev-deploy/deploy.py"
  '';
  meta = {
    description = "Explicit VM-only developer code authority; guarded activation pending";
    mainProgram = "aios-dev-deploy";
    platforms = lib.platforms.linux;
  };
}
