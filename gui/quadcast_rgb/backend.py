"""Animation engine — sends RGB frames to the QuadCast 2S via direct USB.

Runs in a QThread so the GUI stays responsive.  The engine owns the USB
device handle and drives a generator-based animation loop at ~30fps.
"""

from __future__ import annotations

import logging
from collections.abc import Iterator

from PySide6.QtCore import QMutex, QThread, Signal

from quadcast_rgb import animations
from quadcast_rgb.animations import Color, Frame
from quadcast_rgb.errors import DeviceBusyError, DeviceNotFoundError, QuadcastError, USBTransferError
from quadcast_rgb.usb_device import QuadCast2S

log = logging.getLogger(__name__)

TARGET_FPS = 30
FRAME_INTERVAL_MS = int(1000 / TARGET_FPS)


class AnimationEngine(QThread):
    """Background thread that sends animation frames to the QuadCast 2S."""

    frame_sent = Signal(tuple, tuple)  # (upper_color, lower_color) for preview
    error_occurred = Signal(str)
    device_status = Signal(bool)  # True=connected, False=disconnected

    def __init__(self, parent: object = None) -> None:
        super().__init__(parent)
        self._device = QuadCast2S()
        self._generator: Iterator[Frame] | None = None
        self._running = False
        self._mutex = QMutex()

    def configure(
        self,
        mode: str,
        colors: list[Color],
        brightness: int = 100,
        speed: int = 81,
        zone: str = "both",
    ) -> None:
        """Set the animation parameters.  Call start() after to begin."""
        gen_func = animations.MODES.get(mode, animations.solid)
        gen = gen_func(colors=colors, speed=speed, brightness=brightness)

        self._mutex.lock()
        self._generator = _apply_zone_mask(gen, zone)
        self._mutex.unlock()

        log.info("configured: mode=%s zone=%s brightness=%d speed=%d", mode, zone, brightness, speed)

    def run(self) -> None:
        """Thread entry point — open device, run animation loop, cleanup."""
        try:
            self._device.open()
        except DeviceNotFoundError as exc:
            self.error_occurred.emit(str(exc))
            return
        except DeviceBusyError as exc:
            self.error_occurred.emit(str(exc))
            return

        self.device_status.emit(True)
        self._running = True
        log.info("animation loop started")

        try:
            self._animation_loop()
        except USBTransferError as exc:
            log.error("USB transfer failed: %s", exc)
            self.error_occurred.emit(str(exc))
        except QuadcastError as exc:
            log.error("device error: %s", exc)
            self.error_occurred.emit(str(exc))
        finally:
            self._device.close()
            self.device_status.emit(False)
            log.info("animation loop stopped")

    def _animation_loop(self) -> None:
        """Pull frames from generator and send to device until stopped."""
        while self._running:
            self._mutex.lock()
            gen = self._generator
            self._mutex.unlock()

            if gen is None:
                self.msleep(100)
                continue

            frame = next(gen)
            self._device.send_frame(frame.flat)

            # Emit representative colors for the mic preview widget
            upper_c = frame.upper[0] if frame.upper else (0, 0, 0)
            lower_c = frame.lower[0] if frame.lower else (0, 0, 0)
            self.frame_sent.emit(upper_c, lower_c)

            self.msleep(FRAME_INTERVAL_MS)

    def stop(self) -> None:
        """Signal the animation loop to stop and wait for thread exit."""
        self._running = False
        if self.isRunning():
            self.wait(3000)

    @property
    def is_active(self) -> bool:
        return self._running and self.isRunning()


def _apply_zone_mask(gen: Iterator[Frame], zone: str) -> Iterator[Frame]:
    """Wrap a generator to black out LEDs in inactive zones."""
    if zone == "both":
        yield from gen
        return

    black_upper = [(0, 0, 0)] * animations.UPPER_COUNT
    black_lower = [(0, 0, 0)] * animations.LOWER_COUNT

    for frame in gen:
        if zone == "upper":
            yield Frame(upper=frame.upper, lower=black_lower)
        elif zone == "lower":
            yield Frame(upper=black_upper, lower=frame.lower)
        else:
            yield frame
