"""Animation frame generators for QuadCast 2S RGB modes.

Each generator yields Frame objects containing upper (54) and lower (54) LED
color lists.  The animation engine calls next() at ~30fps and sends each
frame to the device.

Speed mapping (matches upstream CLI feel):
  0   = slowest (longest transitions/pauses)
  100 = fastest (shortest transitions/pauses)
"""

from __future__ import annotations

from collections.abc import Iterator
from dataclasses import dataclass, field

Color = tuple[int, int, int]

BLACK: Color = (0, 0, 0)
UPPER_COUNT = 54
LOWER_COUNT = 54
TOTAL_LEDS = UPPER_COUNT + LOWER_COUNT

# Default rainbow for cycle/wave when fewer than 2 colors provided
RAINBOW: list[Color] = [
    (255, 0, 0),
    (255, 0, 158),
    (205, 0, 255),
    (43, 0, 255),
    (0, 104, 255),
    (0, 255, 255),
    (0, 255, 103),
    (50, 255, 0),
    (206, 255, 0),
]


@dataclass(frozen=True, slots=True)
class Frame:
    """One animation frame with separate upper and lower LED arrays."""

    upper: list[Color] = field(default_factory=list)
    lower: list[Color] = field(default_factory=list)

    @property
    def flat(self) -> list[Color]:
        """All 108 LEDs as a single list (upper first, then lower)."""
        return self.upper + self.lower


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def lerp_color(c1: Color, c2: Color, t: float) -> Color:
    """Linear interpolation between two colors.  t clamped to [0, 1]."""
    t = max(0.0, min(1.0, t))
    return (
        int(c1[0] + (c2[0] - c1[0]) * t),
        int(c1[1] + (c2[1] - c1[1]) * t),
        int(c1[2] + (c2[2] - c1[2]) * t),
    )


def apply_brightness(color: Color, brightness: int) -> Color:
    """Scale a color by brightness (0-100)."""
    f = brightness / 100.0
    return (int(color[0] * f), int(color[1] * f), int(color[2] * f))


def _uniform_frame(color: Color) -> Frame:
    """Frame where all LEDs show the same color."""
    return Frame(upper=[color] * UPPER_COUNT, lower=[color] * LOWER_COUNT)


def _build_gradient(colors: list[Color], steps_per_pair: int) -> list[Color]:
    """Pre-compute a looping color gradient through a list of colors."""
    gradient: list[Color] = []
    for i in range(len(colors)):
        c1 = colors[i]
        c2 = colors[(i + 1) % len(colors)]
        for step in range(steps_per_pair):
            gradient.append(lerp_color(c1, c2, step / steps_per_pair))
    return gradient


def _speed_to_gradient_steps(speed: int) -> int:
    """Map speed (0-100) to gradient length per color pair."""
    return max(12, int(128 - speed * 1.16))


def _flash_intensity(frame_num: int, fade_in: int, hold: int, fade_out: int) -> float:
    """Calculate flash intensity (0.0-1.0) at a given frame position."""
    if frame_num < 0:
        return 0.0
    if frame_num < fade_in:
        return frame_num / fade_in
    frame_num -= fade_in
    if frame_num < hold:
        return 1.0
    frame_num -= hold
    if frame_num < fade_out:
        return 1.0 - frame_num / fade_out
    return 0.0


# ---------------------------------------------------------------------------
# Generators — each yields Frame objects forever
# ---------------------------------------------------------------------------


def solid(colors: list[Color], brightness: int = 100, **_: object) -> Iterator[Frame]:
    """Static color on all LEDs."""
    c = apply_brightness(colors[0] if colors else (255, 0, 0), brightness)
    frame = _uniform_frame(c)
    while True:
        yield frame


def blink(
    colors: list[Color],
    speed: int = 81,
    brightness: int = 100,
    **_: object,
) -> Iterator[Frame]:
    """Alternate between colors and black.  speed 0=slow, 100=fast."""
    if not colors:
        colors = [(255, 0, 0)]

    on_frames = max(1, int((101 - speed) * 0.4))
    off_frames = max(1, int(on_frames * 0.4))
    black_frame = _uniform_frame(BLACK)

    color_idx = 0
    while True:
        c = apply_brightness(colors[color_idx % len(colors)], brightness)
        on_frame = _uniform_frame(c)
        for _ in range(on_frames):
            yield on_frame
        for _ in range(off_frames):
            yield black_frame
        color_idx += 1


def cycle(
    colors: list[Color],
    speed: int = 81,
    brightness: int = 100,
    **_: object,
) -> Iterator[Frame]:
    """Smooth gradient cycling through colors.  All LEDs same color per frame."""
    if len(colors) < 2:
        colors = list(RAINBOW)

    steps = _speed_to_gradient_steps(speed)
    gradient = [apply_brightness(c, brightness) for c in _build_gradient(colors, steps)]
    total = len(gradient)

    idx = 0
    while True:
        yield _uniform_frame(gradient[idx % total])
        idx += 1


def wave(
    colors: list[Color],
    speed: int = 81,
    brightness: int = 100,
    **_: object,
) -> Iterator[Frame]:
    """Like cycle but upper/lower zones are offset, creating a wave effect."""
    if len(colors) < 2:
        colors = list(RAINBOW)

    steps = _speed_to_gradient_steps(speed)
    gradient = [apply_brightness(c, brightness) for c in _build_gradient(colors, steps)]
    total = len(gradient)
    offset = total // 2

    idx = 0
    while True:
        upper_c = gradient[idx % total]
        lower_c = gradient[(idx + offset) % total]
        yield Frame(
            upper=[upper_c] * UPPER_COUNT,
            lower=[lower_c] * LOWER_COUNT,
        )
        idx += 1


def lightning(
    colors: list[Color],
    speed: int = 81,
    brightness: int = 100,
    **_: object,
) -> Iterator[Frame]:
    """Lightning flash: black -> fade in -> hold -> fade out -> black.

    Upper zone fires first, lower zone fires with a slight delay.
    """
    if not colors:
        colors = [(255, 0, 0)]

    fade_in = max(2, int(10 - speed * 0.07))
    hold = max(1, int(3 - speed * 0.02))
    fade_out = max(5, int(40 - speed * 0.35))
    pause = max(3, int(20 - speed * 0.17))
    lower_delay = max(1, fade_in // 2)
    total_cycle = fade_in + hold + fade_out + lower_delay + pause

    color_idx = 0
    while True:
        c = apply_brightness(colors[color_idx % len(colors)], brightness)
        for frame_num in range(total_cycle):
            upper_t = _flash_intensity(frame_num, fade_in, hold, fade_out)
            lower_t = _flash_intensity(frame_num - lower_delay, fade_in, hold, fade_out)
            yield Frame(
                upper=[lerp_color(BLACK, c, upper_t)] * UPPER_COUNT,
                lower=[lerp_color(BLACK, c, lower_t)] * LOWER_COUNT,
            )
        color_idx += 1


def pulse(
    colors: list[Color],
    speed: int = 81,
    brightness: int = 100,
    **_: object,
) -> Iterator[Frame]:
    """Synchronized pulse: both zones fade in/out together."""
    if not colors:
        colors = [(255, 0, 0)]

    fade_in = max(2, int(10 - speed * 0.07))
    hold = max(1, int(3 - speed * 0.02))
    fade_out = max(5, int(40 - speed * 0.35))
    pause = max(3, int(20 - speed * 0.17))
    total_cycle = fade_in + hold + fade_out + pause

    color_idx = 0
    while True:
        c = apply_brightness(colors[color_idx % len(colors)], brightness)
        for frame_num in range(total_cycle):
            t = _flash_intensity(frame_num, fade_in, hold, fade_out)
            yield _uniform_frame(lerp_color(BLACK, c, t))
        color_idx += 1


# Mode name -> generator function lookup
MODES: dict[str, type[Iterator[Frame]]] = {
    "solid": solid,
    "blink": blink,
    "cycle": cycle,
    "wave": wave,
    "lightning": lightning,
    "pulse": pulse,
}
