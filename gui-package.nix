{
  lib,
  rustPlatform,
  libusb1,
  pkg-config,
  cmake,
  makeWrapper,
  makeDesktopItem,
  wayland,
  libxkbcommon,
  vulkan-loader,
  libGL,
  fontconfig,
  freetype,
  libx11,
  libxcursor,
  libxrandr,
  libxi,
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

  runtimeLibs = [
    wayland
    libxkbcommon
    vulkan-loader
    libGL
    fontconfig
    freetype
    libx11
    libxcursor
    libxrandr
    libxi
  ];
in
rustPlatform.buildRustPackage {
  pname = "sliglight";
  version = "0.1.0";

  src = ./.;

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [
    pkg-config
    cmake
    makeWrapper
  ];

  buildInputs = [ libusb1 ] ++ runtimeLibs;

  postInstall = ''
    mkdir -p $out/share/applications
    cp ${desktopItem}/share/applications/*.desktop $out/share/applications/

    mkdir -p $out/share/icons/hicolor/scalable/apps
    cp resources/sliglight.svg $out/share/icons/hicolor/scalable/apps/sliglight.svg
  '';

  # iced needs runtime access to Wayland/Vulkan/font libraries
  postFixup = ''
    wrapProgram $out/bin/sliglight \
      --suffix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs}
  '';

  meta = {
    description = "RGB lighting control for HyperX QuadCast microphones";
    homepage = "https://github.com/htelsiz/nix-quadcast";
    license = lib.licenses.gpl2Only;
    platforms = lib.platforms.linux;
    mainProgram = "sliglight";
  };
}
