"""Exception hierarchy for QuadCast RGB control."""


class QuadcastError(Exception):
    """Base exception for all quadcast-rgb errors."""


class DeviceNotFoundError(QuadcastError):
    """QuadCast 2S USB device not found on the bus."""


class DeviceBusyError(QuadcastError):
    """USB device is claimed by another process (e.g. systemd service)."""


class USBTransferError(QuadcastError):
    """USB interrupt transfer failed."""

    def __init__(self, endpoint: int, message: str) -> None:
        self.endpoint = endpoint
        super().__init__(f"EP 0x{endpoint:02x}: {message}")


class ProtocolError(QuadcastError):
    """Device responded with unexpected data."""
