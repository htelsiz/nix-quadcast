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

  # Patches for upstream bugs and improvements:
  # 1. Typo: "desc" should be "descr" (undeclared variable)
  # 2. QuadCast 2S PID 0x02b5 only checked under VID 0x0951 (Kingston),
  #    but HP-branded units report VID 0x03f0. Add 0x02b5 to EU check too.
  # 3. claim_dev_interface gives up on LIBUSB_ERROR_BUSY instead of resetting
  #    the USB device and retrying. This makes RGB control fail whenever any
  #    other program (Wine, PipeWire, etc.) has touched the HID interfaces.
  postPatch = ''
    substituteInPlace modules/devio.c \
      --replace-fail "desc.idProduct" "descr.idProduct"

    substituteInPlace modules/devio.c \
      --replace-fail \
        "descr.idProduct == DEV_PID_DUOCAST)" \
        "descr.idProduct == DEV_PID_DUOCAST || descr.idProduct == DEV_PID_NA3)"

    substituteInPlace modules/devio.c \
      --replace-fail \
        'static int claim_dev_interface(libusb_device_handle *handle)
{
    int errcode0, errcode1;
    libusb_set_auto_detach_kernel_driver(handle, 1); /* might be unsupported */
    errcode0 = libusb_claim_interface(handle, 0);
    errcode1 = libusb_claim_interface(handle, 1);
    if(errcode0 == LIBUSB_ERROR_BUSY || errcode1 == LIBUSB_ERROR_BUSY) {
        fprintf(stderr, BUSY_ERR_MSG);
        return 1;
    } else if(errcode0 == LIBUSB_ERROR_NO_DEVICE ||
                                          errcode1 == LIBUSB_ERROR_NO_DEVICE) {
        fprintf(stderr, OPEN_ERR_MSG);
        return 1;
    }
    return 0;
}' \
        'static int claim_dev_interface(libusb_device_handle *handle)
{
    int errcode0, errcode1;
    libusb_set_auto_detach_kernel_driver(handle, 1);
    errcode0 = libusb_claim_interface(handle, 0);
    errcode1 = libusb_claim_interface(handle, 1);
    if(errcode0 == LIBUSB_ERROR_BUSY || errcode1 == LIBUSB_ERROR_BUSY) {
        /* Reset the USB device to clear stale claims from other programs */
        libusb_release_interface(handle, 0);
        libusb_release_interface(handle, 1);
        if(libusb_reset_device(handle) == 0) {
            libusb_set_auto_detach_kernel_driver(handle, 1);
            errcode0 = libusb_claim_interface(handle, 0);
            errcode1 = libusb_claim_interface(handle, 1);
        }
        if(errcode0 == LIBUSB_ERROR_BUSY || errcode1 == LIBUSB_ERROR_BUSY) {
            fprintf(stderr, BUSY_ERR_MSG);
            return 1;
        }
    }
    if(errcode0 == LIBUSB_ERROR_NO_DEVICE ||
                                          errcode1 == LIBUSB_ERROR_NO_DEVICE) {
        fprintf(stderr, OPEN_ERR_MSG);
        return 1;
    }
    return 0;
}'
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
