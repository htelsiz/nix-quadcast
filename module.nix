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

    color = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "ff0000";
      description = ''
        Hex color to apply on boot via a systemd user service.
        When set, a persistent `quadcast-rgb` service runs `quadcastrgb solid <color>`
        continuously (the mic reverts to default rainbow if the process stops).
        Set to null to disable the service.
      '';
    };

    mode = lib.mkOption {
      type = lib.types.str;
      default = "solid";
      example = "cycle";
      description = "RGB mode: solid, blink, cycle, wave, lightning, or pulse.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages =
      [ quadcastrgb ]
      ++ lib.optionals cfg.enableGui [
        (pkgs.callPackage ./gui-package.nix { inherit quadcastrgb; })
      ];

    # Persistent systemd user service to keep RGB active
    systemd.user.services.quadcast-rgb = lib.mkIf (cfg.color != null) {
      description = "HyperX QuadCast RGB lighting";
      wantedBy = [ "graphical-session.target" ];
      after = [ "graphical-session.target" ];
      serviceConfig = {
        ExecStart = "${quadcastrgb}/bin/quadcastrgb ${cfg.mode} ${cfg.color}";
        Restart = "on-failure";
        RestartSec = 3;
      };
    };

    # udev rules for non-root USB HID access to QuadCast microphones.
    # TAG+="uaccess" grants the logged-in user full device control via systemd-logind,
    # which is required for libusb_set_auto_detach_kernel_driver() to work (needs
    # USBDEVFS_DISCONNECT ioctl permission to unbind usbhid from control interfaces).
    # Covers Kingston (0951) and HP (03f0) vendor IDs across all known models.
    services.udev.extraRules = ''
      # HyperX QuadCast S (Kingston)
      SUBSYSTEM=="usb", ATTR{idVendor}=="0951", ATTR{idProduct}=="171f", MODE="0666", TAG+="uaccess"
      # HyperX QuadCast 2S / DuoCast (HP)
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="0f8b", MODE="0666", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="028c", MODE="0666", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="048c", MODE="0666", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="068c", MODE="0666", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="098c", MODE="0666", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="02b5", MODE="0666", TAG+="uaccess"
      SUBSYSTEM=="usb", ATTR{idVendor}=="03f0", ATTR{idProduct}=="0d84", MODE="0666", TAG+="uaccess"
    '';
  };
}
