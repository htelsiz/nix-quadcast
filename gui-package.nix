{
  lib,
  python3Packages,
  qt6,
  makeDesktopItem,
  quadcastrgb,
}:
let
  desktopItem = makeDesktopItem {
    name = "sliglight";
    desktopName = "Sliglight";
    comment = "RGB lighting control for HyperX QuadCast microphones";
    exec = "sliglight";
    icon = "sliglight";
    categories = [
      "Utility"
      "Settings"
      "HardwareSettings"
    ];
    keywords = [
      "HyperX"
      "QuadCast"
      "RGB"
      "microphone"
    ];
  };
in
python3Packages.buildPythonApplication {
  pname = "sliglight";
  version = "0.1.0";
  pyproject = true;

  src = ./gui;

  build-system = [ python3Packages.setuptools ];

  dependencies = [
    python3Packages.pyside6
    python3Packages.libusb1
  ];

  nativeBuildInputs = [
    qt6.wrapQtAppsHook
    qt6.qtbase
  ];

  # PySide6 needs Qt wrapping applied to Python scripts
  dontWrapQtApps = true;
  makeWrapperArgs = [
    "\${qtWrapperArgs[@]}"
    "--prefix PATH : ${lib.makeBinPath [ quadcastrgb ]}"
  ];

  postInstall = ''
    mkdir -p $out/share/applications
    cp ${desktopItem}/share/applications/*.desktop $out/share/applications/

    mkdir -p $out/share/icons/hicolor/scalable/apps
    cp $src/resources/sliglight.svg $out/share/icons/hicolor/scalable/apps/sliglight.svg
  '';

  # No tests yet
  doCheck = false;

  meta = {
    description = "Qt6 GUI for HyperX QuadCast RGB lighting control";
    homepage = "https://github.com/htelsiz/nix-quadcast";
    license = lib.licenses.gpl2Only;
    platforms = lib.platforms.linux;
    mainProgram = "sliglight";
  };
}
