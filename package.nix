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
    owner = "j-muell";
    repo = "QuadcastRGB2S";
    rev = "a2351e5b77bfe093535e0c6f69590162edcf147d";
    hash = "sha256-ah/6NV3NU9HF8dZQDGpg/+JfoqQ60Bsb+SVy3WEVRAY=";
  };

  buildInputs = [ libusb1 ];

  # Upstream uses void signal handlers incompatible with glibc's sighandler_t.
  # GCC 15+ treats this as an error by default.
  env.NIX_CFLAGS_COMPILE = "-Wno-incompatible-pointer-types";

  # Fix upstream bug: typo in variable name (desc vs descr)
  postPatch = ''
    substituteInPlace modules/devio.c \
      --replace-fail "desc.idProduct" "descr.idProduct"
  '';

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
    homepage = "https://github.com/j-muell/QuadcastRGB2S";
    license = lib.licenses.gpl2Only;
    platforms = lib.platforms.linux;
    mainProgram = "quadcastrgb";
  };
}
