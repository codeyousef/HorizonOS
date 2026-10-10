{ lib, stdenvNoCC, python3, makeWrapper, aiosGuard }:
stdenvNoCC.mkDerivation {
  pname = "aios-dev-deploy";
  version = "0.1.0";
  dontUnpack = true;
  dontBuild = true;
  nativeBuildInputs = [ makeWrapper ];
  installPhase = ''
    mkdir -p $out/lib/aios-dev-deploy $out/bin $out/lib/systemd/system
    cp ${../../tools/guest/dev_deploy.py} $out/lib/aios-dev-deploy/dev_deploy.py
    cp ${../../tools/guest/snapshot.py} $out/lib/aios-dev-deploy/snapshot.py
    cp ${../../tools/guest/build_system.py} $out/lib/aios-dev-deploy/build_system.py
    cp ${./aios-dev-guard-template.service} $out/lib/systemd/system/aios-dev-guard@.service
    substituteInPlace $out/lib/systemd/system/aios-dev-guard@.service \
      --replace-fail @EXECUTABLE@ ${aiosGuard}/bin/aios-guard
    makeWrapper ${python3}/bin/python3 $out/bin/aios-dev-deploy \
      --add-flags "-I $out/lib/aios-dev-deploy/dev_deploy.py"
  '';
  meta = {
    description = "Explicit VM-only developer code authority with independent guarded activation";
    mainProgram = "aios-dev-deploy";
    platforms = lib.platforms.linux;
  };
}
