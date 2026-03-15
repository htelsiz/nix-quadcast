"""Backend for controlling QuadCast RGB via the CLI tool."""

import shutil
import subprocess


def find_cli() -> str:
    path = shutil.which("quadcastrgb")
    if not path:
        raise FileNotFoundError(
            "quadcastrgb not found in PATH. Install nix-quadcast CLI first."
        )
    return path


def build_command(
    mode: str,
    colors: list[str],
    brightness: int,
    speed: int,
    zone: str,
) -> list[str]:
    """Build a quadcastrgb CLI command from GUI state."""
    cmd = [find_cli()]

    # Zone flag
    if zone == "upper":
        cmd.append("-u")
    elif zone == "lower":
        cmd.append("-l")
    # "both" = default (-a), no flag needed

    # Brightness
    cmd.extend(["-b", str(brightness)])

    # Speed (only relevant for animated modes)
    if mode in ("blink", "cycle", "wave", "lightning", "pulse"):
        cmd.extend(["-s", str(speed)])

    # Mode
    cmd.append(mode)

    # Colors (hex without #)
    for color in colors:
        cmd.append(color.lstrip("#"))

    return cmd


def apply(
    mode: str,
    colors: list[str],
    brightness: int,
    speed: int,
    zone: str,
) -> tuple[bool, str]:
    """Apply RGB settings. Returns (success, output)."""
    cmd = build_command(mode, colors, brightness, speed, zone)
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode == 0:
            return True, result.stdout or "Applied successfully"
        return False, result.stderr or f"Exit code {result.returncode}"
    except FileNotFoundError:
        return False, "quadcastrgb not found in PATH"
    except subprocess.TimeoutExpired:
        return False, "Command timed out"
