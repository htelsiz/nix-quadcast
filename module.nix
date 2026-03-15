{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.hardware.quadcast;
  quadcastrgb = pkgs.callPackage ./package.nix { };
in
{
  options.hardware.quadcast = {
    enable = lib.mkEnableOption "HyperX QuadCast RGB control (CLI + udev rules)";

    enableGui = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Install the Qt6 GUI application for RGB control.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages =
      [ quadcastrgb ]
      ++ lib.optionals cfg.enableGui [
        (pkgs.callPackage ./gui-package.nix { inherit quadcastrgb; })
      ];

    # udev rules for non-root USB HID access to QuadCast microphones.
    # Covers Kingston (0951) and HP (03f0) vendor IDs across all known models.
    services.udev.extraRules = ''
      # HyperX QuadCast S (Kingston)
      SUBSYSTEM=="usb", ATTR{idVendor}=="0951", ATTR{idProduct}=="171f", MODE="0666"
      # HyperX QuadCast 2S / DuoCast (HP)
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="0f8b", MODE="0666"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="028c", MODE="0666"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="048c", MODE="0666"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="068c", MODE="0666"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="098c", MODE="0666"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="02b5", MODE="0666"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="0d84", MODE="0666"
    '';
  };
}
