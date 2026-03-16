"""QuadCast RGB GUI — Qt6 controller for HyperX QuadCast microphone RGB."""

import logging
import os
import subprocess
import sys

# Force Qt built-in dialogs instead of KDE native (which ignore our dark theme)
os.environ["QT_QPA_PLATFORMTHEME"] = ""

from PySide6.QtCore import Qt, Signal
from PySide6.QtGui import QColor, QPainter, QPainterPath, QPalette, QRadialGradient
from PySide6.QtWidgets import (
    QApplication,
    QButtonGroup,
    QColorDialog,
    QFrame,
    QGridLayout,
    QHBoxLayout,
    QLabel,
    QMainWindow,
    QMenu,
    QPushButton,
    QSizePolicy,
    QSlider,
    QVBoxLayout,
    QWidget,
)

from quadcast_rgb.backend import AnimationEngine

log = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

MODES = ("solid", "blink", "cycle", "wave", "lightning", "pulse")

MODE_ICONS: dict[str, str] = {
    "solid": "\u25cf",
    "blink": "\u25cc",
    "cycle": "\U0001f308",
    "wave": "\u2248",
    "lightning": "\u26a1",
    "pulse": "\u2665",
}

MODE_DESCRIPTIONS: dict[str, str] = {
    "solid": "Static color",
    "blink": "Blinking effect",
    "cycle": "Rainbow cycle",
    "wave": "Wave animation",
    "lightning": "Lightning strikes",
    "pulse": "Pulsing glow",
}

# --- Catppuccin Mocha palette ---
C_CRUST = "#11111b"
C_MANTLE = "#181825"
C_BASE = "#1e1e2e"
C_SURFACE0 = "#313244"
C_SURFACE1 = "#45475a"
C_SURFACE2 = "#585b70"
C_OVERLAY0 = "#6c7086"
C_SUBTEXT0 = "#a6adc8"
C_TEXT = "#cdd6f4"
C_BLUE = "#89b4fa"
C_MAUVE = "#cba6f7"
C_GREEN = "#a6e3a1"
C_RED = "#f38ba8"
C_PEACH = "#fab387"

STYLESHEET = f"""
    QWidget {{
        background-color: {C_BASE};
        color: {C_TEXT};
        font-size: 13px;
    }}
    QMainWindow {{
        background-color: {C_CRUST};
    }}
    QFrame#card {{
        background-color: {C_MANTLE};
        border: 1px solid {C_SURFACE0};
        border-radius: 10px;
    }}
    QLabel#sectionTitle {{
        font-weight: bold;
        font-size: 11px;
        color: {C_SUBTEXT0};
        letter-spacing: 1px;
        border: none;
        background: transparent;
    }}
    QPushButton {{
        background-color: {C_SURFACE0};
        border: 1px solid {C_SURFACE1};
        border-radius: 6px;
        padding: 8px 12px;
        color: {C_SUBTEXT0};
        min-height: 20px;
    }}
    QPushButton:hover {{
        background-color: {C_SURFACE1};
        color: {C_TEXT};
    }}
    QPushButton:pressed {{
        background-color: {C_SURFACE2};
    }}
    QPushButton:checked {{
        background-color: {C_MAUVE};
        color: {C_CRUST};
        font-weight: bold;
        border-color: {C_MAUVE};
    }}
    QPushButton:checked:hover {{
        background-color: #b4befe;
    }}
    QPushButton#applyButton {{
        background-color: {C_BLUE};
        color: {C_CRUST};
        font-weight: bold;
        font-size: 14px;
        padding: 10px 24px;
        border: none;
        border-radius: 8px;
    }}
    QPushButton#applyButton:hover {{
        background-color: #7ba8f0;
    }}
    QPushButton#applyButton:pressed {{
        background-color: #6b98e0;
    }}
    QPushButton#resetButton {{
        background-color: transparent;
        border: 1px solid {C_SURFACE1};
        color: {C_SUBTEXT0};
    }}
    QPushButton#resetButton:hover {{
        background-color: {C_SURFACE0};
        color: {C_TEXT};
    }}
    QPushButton#addColorButton {{
        font-size: 18px;
        font-weight: bold;
        border: 2px dashed {C_SURFACE1};
        background-color: transparent;
        color: {C_OVERLAY0};
    }}
    QPushButton#addColorButton:hover {{
        background-color: {C_SURFACE0};
        border-color: {C_SUBTEXT0};
        color: {C_TEXT};
    }}
    QSlider::groove:horizontal {{
        background: {C_SURFACE0};
        height: 6px;
        border-radius: 3px;
    }}
    QSlider::sub-page:horizontal {{
        background: qlineargradient(x1:0, y1:0, x2:1, y2:0,
            stop:0 {C_SURFACE1}, stop:1 {C_MAUVE});
        height: 6px;
        border-radius: 3px;
    }}
    QSlider::handle:horizontal {{
        background-color: {C_TEXT};
        border: 3px solid {C_MAUVE};
        width: 16px;
        height: 16px;
        margin: -6px 0;
        border-radius: 11px;
    }}
    QSlider::handle:horizontal:hover {{
        border-color: #b4befe;
        background-color: #ffffff;
    }}
    QToolTip {{
        background-color: {C_SURFACE0};
        color: {C_TEXT};
        border: 1px solid {C_SURFACE1};
        border-radius: 4px;
        padding: 4px 8px;
    }}
"""

SWATCH_CYCLE = (
    "#00ff00", "#0000ff", "#ffff00", "#00ffff", "#ff00ff",
    "#ff8800", "#88ff00", "#ff0088", "#0088ff", "#8800ff",
)

MAX_COLORS = 11
SYSTEMD_SERVICE = "quadcast-rgb.service"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _create_dark_palette() -> QPalette:
    """Catppuccin Mocha QPalette for widgets that ignore QSS."""
    p = QPalette()
    p.setColor(QPalette.ColorRole.Window, QColor(C_BASE))
    p.setColor(QPalette.ColorRole.WindowText, QColor(C_TEXT))
    p.setColor(QPalette.ColorRole.Base, QColor(C_MANTLE))
    p.setColor(QPalette.ColorRole.AlternateBase, QColor(C_SURFACE0))
    p.setColor(QPalette.ColorRole.ToolTipBase, QColor(C_SURFACE0))
    p.setColor(QPalette.ColorRole.ToolTipText, QColor(C_TEXT))
    p.setColor(QPalette.ColorRole.Text, QColor(C_TEXT))
    p.setColor(QPalette.ColorRole.Button, QColor(C_SURFACE0))
    p.setColor(QPalette.ColorRole.ButtonText, QColor(C_TEXT))
    p.setColor(QPalette.ColorRole.BrightText, QColor(C_RED))
    p.setColor(QPalette.ColorRole.Link, QColor(C_BLUE))
    p.setColor(QPalette.ColorRole.Highlight, QColor(C_MAUVE))
    p.setColor(QPalette.ColorRole.HighlightedText, QColor(C_CRUST))
    p.setColor(QPalette.ColorGroup.Disabled, QPalette.ColorRole.WindowText, QColor(C_SURFACE2))
    p.setColor(QPalette.ColorGroup.Disabled, QPalette.ColorRole.Text, QColor(C_SURFACE2))
    p.setColor(QPalette.ColorGroup.Disabled, QPalette.ColorRole.ButtonText, QColor(C_SURFACE2))
    return p


def _stop_systemd_service() -> None:
    """Stop the quadcast-rgb systemd user service so the GUI can claim USB."""
    try:
        result = subprocess.run(
            ["systemctl", "--user", "is-active", "--quiet", SYSTEMD_SERVICE],
            capture_output=True,
        )
        if result.returncode == 0:
            log.info("stopping %s to free USB device", SYSTEMD_SERVICE)
            subprocess.run(
                ["systemctl", "--user", "stop", SYSTEMD_SERVICE],
                capture_output=True,
                check=True,
            )
    except FileNotFoundError:
        log.warning("systemctl not found, cannot check service status")


def _start_systemd_service() -> None:
    """Restart the quadcast-rgb systemd user service."""
    try:
        subprocess.run(
            ["systemctl", "--user", "start", SYSTEMD_SERVICE],
            capture_output=True,
            check=True,
        )
        log.info("restarted %s", SYSTEMD_SERVICE)
    except (FileNotFoundError, subprocess.CalledProcessError) as exc:
        log.warning("failed to restart %s: %s", SYSTEMD_SERVICE, exc)


# ---------------------------------------------------------------------------
# Widgets
# ---------------------------------------------------------------------------


class MicPreview(QWidget):
    """Visual preview of the QuadCast 2S with LED zone glow effects."""

    def __init__(self) -> None:
        super().__init__()
        self.upper_color = QColor("#ff0000")
        self.lower_color = QColor("#ff0000")
        self.setMinimumSize(200, 340)
        self.setSizePolicy(QSizePolicy.Policy.Fixed, QSizePolicy.Policy.Expanding)

    def set_colors(self, upper: QColor, lower: QColor) -> None:
        self.upper_color = upper
        self.lower_color = lower
        self.update()

    def paintEvent(self, event: object) -> None:
        p = QPainter(self)
        p.setRenderHint(QPainter.RenderHint.Antialiasing)
        w, h = self.width(), self.height()
        cx = w / 2

        body_w, body_h, radius = 100, 190, 28
        body_x, body_y = cx - body_w / 2, 40
        stand_h = 70

        # Stand
        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(QColor(C_SURFACE1))
        p.drawRoundedRect(int(cx - 4), int(body_y + body_h - 10), 8, int(stand_h), 4, 4)

        # Base
        p.drawRoundedRect(int(cx - 48), int(body_y + body_h + stand_h - 14), 96, 16, 8, 8)

        # Mic body
        body_path = QPainterPath()
        body_path.addRoundedRect(body_x, body_y, body_w, body_h, radius, radius)
        p.setBrush(QColor("#141420"))
        p.drawPath(body_path)

        # LED zones with glow
        self._paint_led(p, body_x, body_y, body_w, body_h, radius, "upper", self.upper_color)
        self._paint_led(p, body_x, body_y, body_w, body_h, radius, "lower", self.lower_color)

        # Mesh overlay
        p.setClipPath(body_path)
        p.setPen(QColor(0, 0, 0, 40))
        for y_off in range(int(body_y), int(body_y + body_h), 5):
            p.drawLine(int(body_x + 8), y_off, int(body_x + body_w - 8), y_off)
        p.setClipping(False)

        # Mute button
        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(QColor(C_SURFACE0))
        p.drawEllipse(int(cx - 18), int(body_y - 6), 36, 14)

        # Zone labels
        p.setPen(QColor(C_OVERLAY0))
        font = p.font()
        font.setPointSize(9)
        p.setFont(font)
        p.drawText(int(body_x + body_w + 10), int(body_y + body_h * 0.2), "Upper")
        p.drawText(int(body_x + body_w + 10), int(body_y + body_h * 0.7), "Lower")

        p.end()

    def _paint_led(
        self,
        p: QPainter,
        bx: float,
        by: float,
        bw: float,
        bh: float,
        r: float,
        zone: str,
        color: QColor,
    ) -> None:
        p.save()
        pad = 6
        x, zw = bx + pad, bw - pad * 2
        if zone == "upper":
            y, zh = by + pad, bh * 0.38 - pad
        else:
            y, zh = by + bh * 0.42, bh * 0.55 - pad

        zone_path = QPainterPath()
        zone_path.addRoundedRect(x, y, zw, zh, r - pad, r - pad)

        # Outer glow
        glow = QColor(color)
        glow.setAlpha(120)
        glow_end = QColor(color)
        glow_end.setAlpha(0)
        spread = 18
        grad = QRadialGradient(x + zw / 2, y + zh / 2, max(zw, zh))
        grad.setColorAt(0.2, glow)
        grad.setColorAt(1.0, glow_end)
        p.setBrush(grad)
        p.setPen(Qt.PenStyle.NoPen)
        p.drawRoundedRect(x - spread, y - spread, zw + spread * 2, zh + spread * 2, r, r)

        # Solid LED fill
        led_color = QColor(color)
        led_color.setAlpha(200)
        p.setBrush(led_color)
        p.drawPath(zone_path)
        p.restore()


class ColorSwatch(QPushButton):
    """Circular color swatch.  Click to pick, right-click to remove."""

    wants_removal = Signal(object)
    color_changed = Signal()

    def __init__(self, color: str = "#ff0000", removable: bool = True, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._color = QColor(color)
        self._removable = removable
        self.setFixedSize(42, 42)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.clicked.connect(self._pick_color)
        self._update_style()

    @property
    def color(self) -> QColor:
        return self._color

    def _update_style(self) -> None:
        c = self._color.name()
        self.setStyleSheet(f"""
            QPushButton {{
                background-color: {c};
                border: 2px solid {C_SURFACE1};
                border-radius: 21px;
            }}
            QPushButton:hover {{
                border-color: {C_TEXT};
            }}
        """)
        tip = c.upper()
        if self._removable:
            tip += "\nRight-click to remove"
        self.setToolTip(tip)

    def _pick_color(self) -> None:
        dialog = QColorDialog(self._color, self)
        dialog.setOption(QColorDialog.ColorDialogOption.DontUseNativeDialog)
        if dialog.exec() == QColorDialog.DialogCode.Accepted:
            self._color = dialog.selectedColor()
            self._update_style()
            self.color_changed.emit()

    def contextMenuEvent(self, event: object) -> None:
        if not self._removable:
            return
        menu = QMenu(self)
        menu.addAction("Remove Color").triggered.connect(lambda: self.wants_removal.emit(self))
        menu.exec(self.mapToGlobal(event.pos()))


class Card(QFrame):
    """Styled card container with section title."""

    def __init__(self, title: str = "", parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setObjectName("card")
        self._layout = QVBoxLayout(self)
        self._layout.setContentsMargins(16, 14, 16, 14)
        self._layout.setSpacing(10)
        if title:
            lbl = QLabel(title.upper())
            lbl.setObjectName("sectionTitle")
            self._layout.addWidget(lbl)

    def content(self) -> QVBoxLayout:
        return self._layout


class LabeledSlider(QWidget):
    """Label + horizontal slider + value display."""

    def __init__(self, label: str, min_v: int, max_v: int, default: int) -> None:
        super().__init__()
        row = QHBoxLayout(self)
        row.setContentsMargins(0, 0, 0, 0)
        row.setSpacing(10)

        lbl = QLabel(label)
        lbl.setFixedWidth(72)
        lbl.setStyleSheet(f"color: {C_SUBTEXT0}; background: transparent; border: none;")
        row.addWidget(lbl)

        self.slider = QSlider(Qt.Orientation.Horizontal)
        self.slider.setRange(min_v, max_v)
        self.slider.setValue(default)
        self.slider.setCursor(Qt.CursorShape.PointingHandCursor)
        row.addWidget(self.slider, stretch=1)

        self.val_label = QLabel(str(default))
        self.val_label.setFixedWidth(32)
        self.val_label.setAlignment(Qt.AlignmentFlag.AlignRight)
        self.val_label.setStyleSheet(f"color: {C_TEXT}; font-weight: bold; background: transparent; border: none;")
        row.addWidget(self.val_label)

        self.slider.valueChanged.connect(lambda v: self.val_label.setText(str(v)))

    def value(self) -> int:
        return self.slider.value()

    def setValue(self, v: int) -> None:
        self.slider.setValue(v)


# ---------------------------------------------------------------------------
# Main window
# ---------------------------------------------------------------------------


class MainWindow(QMainWindow):
    """QuadCast RGB control panel."""

    def __init__(self) -> None:
        super().__init__()
        self.setWindowTitle("QuadCast RGB")
        self.setMinimumSize(620, 700)

        # Animation engine (runs in background thread)
        self.engine = AnimationEngine(self)
        self.engine.frame_sent.connect(self._on_frame)
        self.engine.error_occurred.connect(self._on_error)
        self.engine.device_status.connect(self._on_device_status)

        # Build UI
        central = QWidget()
        self.setCentralWidget(central)
        main_layout = QHBoxLayout(central)
        main_layout.setContentsMargins(20, 20, 20, 20)
        main_layout.setSpacing(20)

        # Left: mic preview
        self.mic_preview = MicPreview()
        main_layout.addWidget(
            self.mic_preview,
            alignment=Qt.AlignmentFlag.AlignTop | Qt.AlignmentFlag.AlignHCenter,
        )

        # Right: controls
        controls = QVBoxLayout()
        controls.setSpacing(14)
        main_layout.addLayout(controls, stretch=1)

        self._build_zone_card(controls)
        self._build_mode_card(controls)
        self._build_settings_card(controls)
        self._build_colors_card(controls)
        controls.addStretch(1)
        self._build_actions(controls)

        # Status bar
        self.status_label = QLabel("")
        self.status_label.setStyleSheet(
            f"font-size: 11px; color: {C_OVERLAY0}; background: transparent; border: none;"
        )
        self.status_label.setWordWrap(True)
        controls.addWidget(self.status_label)

        self.zone_group.buttonClicked.connect(self._update_preview)
        self._reset()

    # --- UI builders ---

    def _build_zone_card(self, parent: QVBoxLayout) -> None:
        card = Card("Zone")
        row = QHBoxLayout()
        row.setSpacing(6)
        card.content().addLayout(row)

        self.zone_group = QButtonGroup(self)
        self.zone_group.setExclusive(True)
        self.zone_buttons: dict[str, QPushButton] = {}
        for name, text in [("both", "Both"), ("upper", "Upper"), ("lower", "Lower")]:
            btn = QPushButton(text)
            btn.setCheckable(True)
            btn.setCursor(Qt.CursorShape.PointingHandCursor)
            self.zone_group.addButton(btn)
            self.zone_buttons[name] = btn
            row.addWidget(btn)
        parent.addWidget(card)

    def _build_mode_card(self, parent: QVBoxLayout) -> None:
        card = Card("Mode")
        grid = QGridLayout()
        grid.setSpacing(6)
        card.content().addLayout(grid)

        self.mode_group = QButtonGroup(self)
        self.mode_group.setExclusive(True)
        self.mode_buttons: dict[str, QPushButton] = {}
        for i, mode in enumerate(MODES):
            btn = QPushButton(f"{MODE_ICONS[mode]}  {mode.capitalize()}")
            btn.setCheckable(True)
            btn.setCursor(Qt.CursorShape.PointingHandCursor)
            btn.setToolTip(MODE_DESCRIPTIONS[mode])
            self.mode_group.addButton(btn)
            self.mode_buttons[mode] = btn
            grid.addWidget(btn, i // 3, i % 3)
        parent.addWidget(card)

    def _build_settings_card(self, parent: QVBoxLayout) -> None:
        card = Card("Settings")
        self.brightness_slider = LabeledSlider("Brightness", 0, 100, 80)
        self.speed_slider = LabeledSlider("Speed", 0, 100, 81)
        card.content().addWidget(self.brightness_slider)
        card.content().addWidget(self.speed_slider)
        parent.addWidget(card)

    def _build_colors_card(self, parent: QVBoxLayout) -> None:
        card = Card("Colors")
        self.color_row = QHBoxLayout()
        self.color_row.setSpacing(8)
        card.content().addLayout(self.color_row)

        self.color_swatches: list[ColorSwatch] = []

        self.add_color_btn = QPushButton("+")
        self.add_color_btn.setObjectName("addColorButton")
        self.add_color_btn.setFixedSize(42, 42)
        self.add_color_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self.add_color_btn.setToolTip(f"Add color (max {MAX_COLORS})")
        self.add_color_btn.clicked.connect(lambda: self._add_swatch())

        self.color_row.addStretch()
        parent.addWidget(card)

    def _build_actions(self, parent: QVBoxLayout) -> None:
        row = QHBoxLayout()
        row.setSpacing(10)

        reset_btn = QPushButton("Reset")
        reset_btn.setObjectName("resetButton")
        reset_btn.setMinimumHeight(40)
        reset_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        reset_btn.clicked.connect(self._reset)
        row.addWidget(reset_btn, stretch=1)

        apply_btn = QPushButton("Apply")
        apply_btn.setObjectName("applyButton")
        apply_btn.setMinimumHeight(40)
        apply_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        apply_btn.clicked.connect(self._apply)
        row.addWidget(apply_btn, stretch=2)

        parent.addLayout(row)

    # --- Color swatch management ---

    def _add_swatch(self, color: str = "#00ff00", removable: bool = True) -> None:
        if len(self.color_swatches) >= MAX_COLORS:
            return
        if removable and self.color_swatches:
            color = SWATCH_CYCLE[(len(self.color_swatches) - 1) % len(SWATCH_CYCLE)]

        swatch = ColorSwatch(color, removable)
        swatch.wants_removal.connect(self._remove_swatch)
        swatch.color_changed.connect(self._update_preview)
        self.color_swatches.append(swatch)
        self.color_row.insertWidget(len(self.color_swatches) - 1, swatch)
        self._sync_add_btn()

    def _remove_swatch(self, swatch: ColorSwatch) -> None:
        if swatch in self.color_swatches:
            self.color_swatches.remove(swatch)
            self.color_row.removeWidget(swatch)
            swatch.deleteLater()
            self._sync_add_btn()
            self._update_preview()

    def _sync_add_btn(self) -> None:
        full = len(self.color_swatches) >= MAX_COLORS
        self.add_color_btn.setVisible(not full)
        self.color_row.removeWidget(self.add_color_btn)
        self.color_row.insertWidget(len(self.color_swatches), self.add_color_btn)

    # --- State readers ---

    def _get_zone(self) -> str:
        for name, btn in self.zone_buttons.items():
            if btn.isChecked():
                return name
        return "both"

    def _get_mode(self) -> str:
        for name, btn in self.mode_buttons.items():
            if btn.isChecked():
                return name
        return "solid"

    def _get_colors_rgb(self) -> list[tuple[int, int, int]]:
        """Extract (R, G, B) tuples from swatches."""
        return [
            (s.color.red(), s.color.green(), s.color.blue())
            for s in self.color_swatches
        ]

    # --- Preview ---

    def _update_preview(self, _: object = None) -> None:
        if not self.color_swatches:
            return
        c = self.color_swatches[0].color
        zone = self._get_zone()
        off = QColor("#141420")
        if zone == "both":
            self.mic_preview.set_colors(c, c)
        elif zone == "upper":
            self.mic_preview.set_colors(c, off)
        elif zone == "lower":
            self.mic_preview.set_colors(off, c)

    def _on_frame(self, upper: tuple[int, int, int], lower: tuple[int, int, int]) -> None:
        """Called from engine thread — update mic preview with live device colors."""
        self.mic_preview.set_colors(
            QColor(upper[0], upper[1], upper[2]),
            QColor(lower[0], lower[1], lower[2]),
        )

    def _on_error(self, message: str) -> None:
        log.error("engine error: %s", message)
        self.status_label.setText(f"Error: {message}")
        self.status_label.setStyleSheet(
            f"font-size: 11px; color: {C_RED}; background: transparent; border: none;"
        )

    def _on_device_status(self, connected: bool) -> None:
        if connected:
            self.status_label.setText("Connected to QuadCast 2S")
            self.status_label.setStyleSheet(
                f"font-size: 11px; color: {C_GREEN}; background: transparent; border: none;"
            )
        else:
            self.status_label.setText("Disconnected")
            self.status_label.setStyleSheet(
                f"font-size: 11px; color: {C_OVERLAY0}; background: transparent; border: none;"
            )

    # --- Actions ---

    def _apply(self) -> None:
        mode = self._get_mode()
        zone = self._get_zone()
        brightness = self.brightness_slider.value()
        speed = self.speed_slider.value()
        colors = self._get_colors_rgb()

        # Stop any running animation first
        if self.engine.is_active:
            self.engine.stop()

        self._set_status(f"Applying {mode}...", C_OVERLAY0)

        # Stop systemd service so we can claim the USB device
        _stop_systemd_service()

        self.engine.configure(mode=mode, colors=colors, brightness=brightness, speed=speed, zone=zone)
        self.engine.start()

    def _reset(self) -> None:
        self.zone_buttons["both"].setChecked(True)
        self.mode_buttons["solid"].setChecked(True)
        self.brightness_slider.setValue(80)
        self.speed_slider.setValue(81)

        while self.color_swatches:
            s = self.color_swatches.pop()
            self.color_row.removeWidget(s)
            s.deleteLater()

        self._add_swatch("#ff0000", removable=False)
        self._sync_add_btn()
        self.status_label.setText("")
        self._update_preview()

    def _set_status(self, text: str, color: str) -> None:
        self.status_label.setText(text)
        self.status_label.setStyleSheet(
            f"font-size: 11px; color: {color}; background: transparent; border: none;"
        )

    def closeEvent(self, event: object) -> None:
        """Clean up engine on window close, restart systemd service."""
        if self.engine.is_active:
            self.engine.stop()
        _start_systemd_service()
        super().closeEvent(event)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format="%(name)s: %(message)s",
    )

    app = QApplication(sys.argv)
    app.setStyle("Fusion")
    app.setPalette(_create_dark_palette())
    app.setStyleSheet(STYLESHEET)

    window = MainWindow()
    window.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
