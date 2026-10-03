{ lib, stdenv, cmake, qt6, testing ? false }:
stdenv.mkDerivation {
  pname = if testing then "aios-consent-ui-tests" else "aios-consent-ui";
  version = "0.1.0";
  src = ../../desktop/consent;
  nativeBuildInputs = [ cmake qt6.wrapQtAppsHook ];
  buildInputs = [ qt6.qtbase qt6.qtwayland ];
  cmakeFlags = [ "-DBUILD_TESTING=${if testing then "ON" else "OFF"}" ];
  doCheck = testing;
  # Fixture widget tests run offscreen in the builder. The same test binary is
  # separately run on the explicitly selected real Wayland desktop.
  checkPhase = ''QT_QPA_PLATFORM=offscreen ctest --output-on-failure'';
  meta.mainProgram = "aios-scope-dialog";
  meta.license = lib.licenses.mit;
}
