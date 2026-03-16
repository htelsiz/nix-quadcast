"""Direct USB communication with HyperX QuadCast 2S for RGB control.

Protocol (discovered via Wireshark capture of NGENUITY):
  - All communication on Interface 1: EP 0x06 OUT, EP 0x85 IN
  - Each OUT write must be followed by an IN read (ACK)
  - Frame = 1 header packet + 6 data packets (108 LEDs, 20 per packet)
  - Header: 44 01 06 00 [zeros...]
  - Data:   44 02 <idx> 00 <R G B x 20>
"""

import logging

import usb1

from quadcast_rgb.errors import DeviceBusyError, DeviceNotFoundError, USBTransferError

log = logging.getLogger(__name__)

# Device identifiers
VID = 0x03F0  # HP (HyperX)
PIDS = frozenset({0x02B5})  # QuadCast 2S controller

# USB endpoints — Interface 1
EP_OUT = 0x06
EP_IN = 0x85
IFACE = 1

# Protocol constants
PKT_SIZE = 64
NUM_LEDS = 108
LEDS_PER_PKT = 20
NUM_DATA_PKTS = 6
TIMEOUT_MS = 1000

# Packet codes
HEADER_CMD = 0x44
HEADER_SUB = 0x01
DATA_CMD = 0x44
DATA_SUB = 0x02
PKT_COUNT_CODE = 0x06

Color = tuple[int, int, int]


class QuadCast2S:
    """Direct USB interface to QuadCast 2S RGB LEDs."""

    def __init__(self) -> None:
        self._ctx: usb1.USBContext | None = None
        self._handle: usb1.USBDeviceHandle | None = None
        self._claimed: bool = False

    @property
    def is_open(self) -> bool:
        return self._handle is not None and self._claimed

    def open(self) -> None:
        """Find and open the QuadCast 2S, claim interface 1.

        Raises:
            DeviceNotFoundError: Device not on USB bus.
            DeviceBusyError: Interface claimed by another process.
        """
        self._ctx = usb1.USBContext()

        for dev in self._ctx.getDeviceList():
            if dev.getVendorID() == VID and dev.getProductID() in PIDS:
                self._handle = dev.open()
                try:
                    if self._handle.kernelDriverActive(IFACE):
                        self._handle.detachKernelDriver(IFACE)
                    self._handle.claimInterface(IFACE)
                except usb1.USBErrorBusy as exc:
                    self._handle.close()
                    self._handle = None
                    raise DeviceBusyError(
                        "Interface 1 is claimed by another process "
                        "(stop quadcast-rgb.service first)"
                    ) from exc

                self._claimed = True
                log.info("opened QuadCast 2S (VID=%04x PID=%04x)", VID, dev.getProductID())
                return

        raise DeviceNotFoundError("QuadCast 2S not found on USB bus")

    def close(self) -> None:
        """Release interface and close device. Safe to call multiple times."""
        if self._handle and self._claimed:
            try:
                self._handle.releaseInterface(IFACE)
            except usb1.USBError:
                pass
            self._claimed = False

        if self._handle:
            try:
                self._handle.close()
            except usb1.USBError:
                pass
            self._handle = None

        if self._ctx:
            self._ctx.close()
            self._ctx = None

        log.info("closed QuadCast 2S")

    def send_frame(self, leds: list[Color]) -> None:
        """Send a full 108-LED frame to the device.

        Args:
            leds: List of (R, G, B) tuples, values 0-255.
                  Padded to 108 if shorter.

        Raises:
            USBTransferError: Write or ACK read failed.
        """
        if not self.is_open:
            raise USBTransferError(EP_OUT, "device not open")

        # Pad to 108 LEDs
        frame = list(leds)
        while len(frame) < NUM_LEDS:
            frame.append((0, 0, 0))

        # Header packet: 44 01 06 00 [zeros]
        header = bytearray(PKT_SIZE)
        header[0] = HEADER_CMD
        header[1] = HEADER_SUB
        header[2] = PKT_COUNT_CODE

        self._send_recv(header)

        # 6 data packets
        led_idx = 0
        for pkt_num in range(NUM_DATA_PKTS):
            data = bytearray(PKT_SIZE)
            data[0] = DATA_CMD
            data[1] = DATA_SUB
            data[2] = pkt_num

            offset = 4
            for _ in range(LEDS_PER_PKT):
                if led_idx < NUM_LEDS:
                    r, g, b = frame[led_idx]
                    data[offset] = r & 0xFF
                    data[offset + 1] = g & 0xFF
                    data[offset + 2] = b & 0xFF
                    led_idx += 1
                offset += 3

            self._send_recv(data)

    def _send_recv(self, packet: bytearray) -> bytes:
        """Write packet to EP_OUT, read ACK from EP_IN.

        Raises:
            USBTransferError: On write or read failure.
        """
        assert self._handle is not None

        try:
            self._handle.interruptWrite(EP_OUT, bytes(packet), TIMEOUT_MS)
        except usb1.USBError as exc:
            raise USBTransferError(EP_OUT, str(exc)) from exc

        try:
            return self._handle.interruptRead(EP_IN, PKT_SIZE, TIMEOUT_MS)
        except usb1.USBError as exc:
            raise USBTransferError(EP_IN, f"ACK read failed: {exc}") from exc
