{
  lib,
  stdenv,
  fetchFromGitHub,
  libusb1,
}:
stdenv.mkDerivation {
  pname = "quadcastrgb";
  version = "1.0.4-unstable-2025-04-14";

  src = fetchFromGitHub {
    owner = "Ors1mer";
    repo = "QuadcastRGB";
    rev = "1bd83c7ed8a57dfacce93228e6de40fb552162fd"; # support-for-quadcast-2s branch
    hash = "sha256-mu8Azly1aM2B/nVNCaAqSBFDQ9y2rIT1NT9Zv38LIT4=";
  };

  buildInputs = [ libusb1 ];

  # Reset USB device on BUSY instead of giving up.
  # Other programs (Wine, PipeWire, etc.) may leave stale HID claims.
  patches = [ ./usb-reset-on-busy.patch ];

  buildPhase = ''
    runHook preBuild
    make quadcastrgb
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin $out/share/man/man1
    cp quadcastrgb $out/bin/
    cp man/quadcastrgb.1.gz $out/share/man/man1/ 2>/dev/null || true
    runHook postInstall
  '';

  meta = {
    description = "RGB lighting control for HyperX QuadCast S, QuadCast 2S, and DuoCast microphones";
    homepage = "https://github.com/Ors1mer/QuadcastRGB";
    license = lib.licenses.gpl2Only;
    platforms = lib.platforms.linux;
    mainProgram = "quadcastrgb";
  };
}
