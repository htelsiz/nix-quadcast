"""QuadCast RGB GUI — Qt6 controller for HyperX QuadCast microphone RGB."""

import sys

from PySide6.QtCore import Qt, QSize
from PySide6.QtGui import QColor, QIcon, QPainter, QPainterPath, QLinearGradient
from PySide6.QtWidgets import (
    QApplication,
    QColorDialog,
    QFrame,
    QGridLayout,
    QHBoxLayout,
    QLabel,
    QMainWindow,
    QPushButton,
    QSlider,
    QSizePolicy,
    QVBoxLayout,
    QWidget,
    QButtonGroup,
    QMessageBox,
    QGraphicsDropShadowEffect,
)

from quadcast_rgb import backend

MODES = ["solid", "blink", "cycle", "wave", "lightning", "pulse"]
MODE_ICONS = {
    "solid": "\u25cf",      # filled circle
    "blink": "\u2733",      # sparkle
    "cycle": "\U0001f308",  # rainbow
    "wave": "\u223f",       # sine wave
    "lightning": "\u26a1",  # lightning
    "pulse": "\u2764",      # heart
}
MODE_DESCRIPTIONS = {
    "solid": "Static color",
    "blink": "Blinking effect",
    "cycle": "Rainbow cycle",
    "wave": "Wave animation",
    "lightning": "Lightning strikes",
    "pulse": "Pulsing glow",
}


class MicPreview(QWidget):
    """Visual preview of the QuadCast 2S with upper and lower LED zones."""

    def __init__(self):
        super().__init__()
        self.upper_color = QColor("#ff0000")
        self.lower_color = QColor("#ff0000")
        self.setMinimumSize(160, 280)
        self.setSizePolicy(QSizePolicy.Policy.Fixed, QSizePolicy.Policy.Fixed)

    def set_upper_color(self, color: QColor):
        self.upper_color = color
        self.update()

    def set_lower_color(self, color: QColor):
        self.lower_color = color
        self.update()

    def paintEvent(self, event):
        p = QPainter(self)
        p.setRenderHint(QPainter.RenderHint.Antialiasing)
        w, h = self.width(), self.height()
        cx = w / 2

        # Mic body dimensions
        body_w = 90
        body_h = 160
        body_x = cx - body_w / 2
        body_y = 30
        radius = 20

        # Stand
        stand_w = 6
        stand_h = 60
        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(QColor("#555555"))
        p.drawRoundedRect(
            int(cx - stand_w / 2),
            int(body_y + body_h),
            int(stand_w),
            int(stand_h),
            3, 3,
        )

        # Base
        base_w = 80
        base_h = 14
        p.drawRoundedRect(
            int(cx - base_w / 2),
            int(body_y + body_h + stand_h - 2),
            int(base_w),
            int(base_h),
            7, 7,
        )

        # Mic body (dark shell)
        p.setBrush(QColor("#1a1a1a"))
        p.drawRoundedRect(
            int(body_x), int(body_y), int(body_w), int(body_h),
            radius, radius,
        )

        # Upper LED zone (top third of mic body)
        upper_zone = QPainterPath()
        upper_zone.addRoundedRect(
            body_x + 4, body_y + 4, body_w - 8, body_h * 0.35,
            radius - 2, radius - 2,
        )
        glow_upper = QColor(self.upper_color)
        glow_upper.setAlpha(180)
        p.setBrush(glow_upper)
        p.drawPath(upper_zone)

        # Lower LED zone (bottom two-thirds)
        lower_zone = QPainterPath()
        lower_zone.addRoundedRect(
            body_x + 4, body_y + body_h * 0.4, body_w - 8, body_h * 0.55,
            radius - 2, radius - 2,
        )
        glow_lower = QColor(self.lower_color)
        glow_lower.setAlpha(180)
        p.setBrush(glow_lower)
        p.drawPath(lower_zone)

        # Mesh pattern overlay (horizontal lines to look like grille)
        p.setPen(QColor(0, 0, 0, 60))
        for y_off in range(0, int(body_h - 10), 6):
            y_pos = int(body_y + 8 + y_off)
            p.drawLine(int(body_x + 12), y_pos, int(body_x + body_w - 12), y_pos)

        # Mute button on top
        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(QColor("#333333"))
        p.drawEllipse(int(cx - 15), int(body_y - 5), 30, 12)

        # Labels
        p.setPen(QColor("#888888"))
        font = p.font()
        font.setPointSize(8)
        p.setFont(font)
        p.drawText(int(body_x + body_w + 8), int(body_y + body_h * 0.18), "Upper")
        p.drawText(int(body_x + body_w + 8), int(body_y + body_h * 0.65), "Lower")

        p.end()


class ColorSwatch(QPushButton):
    """Clickable color swatch that opens a color picker."""

    def __init__(self, color: str = "#ff0000", parent=None):
        super().__init__(parent)
        self._color = QColor(color)
        self.setFixedSize(44, 44)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.clicked.connect(self._pick_color)
        self._update_style()

    @property
    def color(self) -> QColor:
        return self._color

    def set_color(self, color: QColor):
        self._color = color
        self._update_style()

    def _update_style(self):
        hex_color = self._color.name()
        self.setStyleSheet(
            f"background-color: {hex_color};"
            f"border: 2px solid #666;"
            f"border-radius: 8px;"
        )
        self.setToolTip(hex_color.upper())

    def _pick_color(self):
        color = QColorDialog.getColor(
            self._color,
            self,
            "Pick LED Color",
            QColorDialog.ColorDialogOption.ShowAlphaChannel
            if False
            else QColorDialog.ColorDialogOption(0),
        )
        if color.isValid():
            self._color = color
            self._update_style()


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("QuadCast RGB")
        self.setMinimumSize(520, 620)
        self.setMaximumSize(700, 800)

        central = QWidget()
        self.setCentralWidget(central)
        layout = QHBoxLayout(central)
        layout.setContentsMargins(20, 20, 20, 20)
        layout.setSpacing(20)

        # Left: mic preview
        self.mic_preview = MicPreview()
        layout.addWidget(self.mic_preview, alignment=Qt.AlignmentFlag.AlignTop)

        # Right: controls
        controls = QVBoxLayout()
        controls.setSpacing(14)
        layout.addLayout(controls, stretch=1)

        # --- Zone selector ---
        zone_label = QLabel("Zone")
        zone_label.setStyleSheet("font-weight: bold; font-size: 13px;")
        controls.addWidget(zone_label)

        zone_row = QHBoxLayout()
        self.zone_group = QButtonGroup(self)
        self.zone_group.setExclusive(True)
        self.zone_buttons = {}
        for zone_name, zone_text in [("both", "Both"), ("upper", "Upper"), ("lower", "Lower")]:
            btn = QPushButton(zone_text)
            btn.setCheckable(True)
            btn.setMinimumHeight(32)
            btn.setCursor(Qt.CursorShape.PointingHandCursor)
            self.zone_group.addButton(btn)
            self.zone_buttons[zone_name] = btn
            zone_row.addWidget(btn)
        self.zone_buttons["both"].setChecked(True)
        controls.addLayout(zone_row)

        # --- Mode selector ---
        mode_label = QLabel("Mode")
        mode_label.setStyleSheet("font-weight: bold; font-size: 13px;")
        controls.addWidget(mode_label)

        mode_grid = QGridLayout()
        mode_grid.setSpacing(6)
        self.mode_group = QButtonGroup(self)
        self.mode_group.setExclusive(True)
        self.mode_buttons = {}
        for i, mode in enumerate(MODES):
            btn = QPushButton(f"{MODE_ICONS[mode]}  {mode.capitalize()}")
            btn.setCheckable(True)
            btn.setMinimumHeight(36)
            btn.setCursor(Qt.CursorShape.PointingHandCursor)
            btn.setToolTip(MODE_DESCRIPTIONS[mode])
            self.mode_group.addButton(btn)
            self.mode_buttons[mode] = btn
            mode_grid.addWidget(btn, i // 3, i % 3)
        self.mode_buttons["solid"].setChecked(True)
        controls.addLayout(mode_grid)

        # --- Brightness ---
        self._add_slider(controls, "Brightness", 0, 100, 80, "brightness")

        # --- Speed ---
        self._add_slider(controls, "Speed", 0, 100, 81, "speed")

        # --- Colors ---
        color_label = QLabel("Colors")
        color_label.setStyleSheet("font-weight: bold; font-size: 13px;")
        controls.addWidget(color_label)

        color_row = QHBoxLayout()
        color_row.setSpacing(8)
        self.color_swatches: list[ColorSwatch] = []

        # Start with one red swatch
        swatch = ColorSwatch("#ff0000")
        self.color_swatches.append(swatch)
        color_row.addWidget(swatch)

        # Add button
        self.add_color_btn = QPushButton("+")
        self.add_color_btn.setFixedSize(44, 44)
        self.add_color_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self.add_color_btn.setStyleSheet(
            "font-size: 20px; font-weight: bold; border: 2px dashed #666; border-radius: 8px;"
        )
        self.add_color_btn.setToolTip("Add color (max 11)")
        self.add_color_btn.clicked.connect(self._add_color)
        color_row.addWidget(self.add_color_btn)

        color_row.addStretch()
        self.color_layout = color_row
        controls.addLayout(color_row)

        # --- Separator ---
        line = QFrame()
        line.setFrameShape(QFrame.Shape.HLine)
        line.setFrameShadow(QFrame.Shadow.Sunken)
        controls.addWidget(line)

        # --- Apply / Reset ---
        action_row = QHBoxLayout()

        self.apply_btn = QPushButton("Apply")
        self.apply_btn.setMinimumHeight(40)
        self.apply_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self.apply_btn.setStyleSheet(
            "QPushButton { background-color: #2563eb; color: white; font-weight: bold;"
            " font-size: 14px; border-radius: 8px; padding: 8px 24px; }"
            "QPushButton:hover { background-color: #1d4ed8; }"
            "QPushButton:pressed { background-color: #1e40af; }"
        )
        self.apply_btn.clicked.connect(self._apply)
        action_row.addWidget(self.apply_btn, stretch=2)

        self.reset_btn = QPushButton("Reset")
        self.reset_btn.setMinimumHeight(40)
        self.reset_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self.reset_btn.clicked.connect(self._reset)
        action_row.addWidget(self.reset_btn, stretch=1)

        controls.addLayout(action_row)

        # Status
        self.status_label = QLabel("")
        self.status_label.setStyleSheet("font-size: 11px; color: #888;")
        self.status_label.setWordWrap(True)
        controls.addWidget(self.status_label)

        controls.addStretch()

        # Update preview on any change
        self.zone_group.buttonClicked.connect(self._update_preview)
        self.mode_group.buttonClicked.connect(self._update_preview)
        self._update_preview()

    def _add_slider(self, layout, label_text, min_val, max_val, default, attr_name):
        row = QHBoxLayout()
        label = QLabel(label_text)
        label.setStyleSheet("font-weight: bold; font-size: 13px;")
        label.setFixedWidth(80)
        row.addWidget(label)

        slider = QSlider(Qt.Orientation.Horizontal)
        slider.setRange(min_val, max_val)
        slider.setValue(default)
        slider.setCursor(Qt.CursorShape.PointingHandCursor)
        row.addWidget(slider, stretch=1)

        value_label = QLabel(str(default))
        value_label.setFixedWidth(30)
        value_label.setAlignment(Qt.AlignmentFlag.AlignRight)
        row.addWidget(value_label)

        slider.valueChanged.connect(lambda v: value_label.setText(str(v)))
        layout.addLayout(row)
        setattr(self, f"{attr_name}_slider", slider)

    def _add_color(self):
        if len(self.color_swatches) >= 11:
            return
        swatch = ColorSwatch("#00ff00" if len(self.color_swatches) == 1 else "#0000ff")
        self.color_swatches.append(swatch)
        # Insert before the + button
        idx = self.color_layout.indexOf(self.add_color_btn)
        self.color_layout.insertWidget(idx, swatch)
        if len(self.color_swatches) >= 11:
            self.add_color_btn.setEnabled(False)

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

    def _update_preview(self, _=None):
        """Update the mic preview to reflect current swatch colors."""
        if not self.color_swatches:
            return
        first_color = self.color_swatches[0].color
        zone = self._get_zone()
        if zone == "both":
            self.mic_preview.set_upper_color(first_color)
            self.mic_preview.set_lower_color(first_color)
        elif zone == "upper":
            self.mic_preview.set_upper_color(first_color)
        elif zone == "lower":
            self.mic_preview.set_lower_color(first_color)

    def _apply(self):
        mode = self._get_mode()
        zone = self._get_zone()
        brightness = self.brightness_slider.value()
        speed = self.speed_slider.value()
        colors = [s.color.name().lstrip("#") for s in self.color_swatches]

        self.status_label.setText("Applying...")
        self.status_label.setStyleSheet("font-size: 11px; color: #888;")

        success, msg = backend.apply(mode, colors, brightness, speed, zone)

        if success:
            self.status_label.setText(f"Applied: {mode} ({zone})")
            self.status_label.setStyleSheet("font-size: 11px; color: #22c55e;")
            self._update_preview()
        else:
            self.status_label.setText(f"Error: {msg}")
            self.status_label.setStyleSheet("font-size: 11px; color: #ef4444;")

    def _reset(self):
        self.zone_buttons["both"].setChecked(True)
        self.mode_buttons["solid"].setChecked(True)
        self.brightness_slider.setValue(80)
        self.speed_slider.setValue(81)
        # Remove extra swatches
        while len(self.color_swatches) > 1:
            swatch = self.color_swatches.pop()
            self.color_layout.removeWidget(swatch)
            swatch.deleteLater()
        self.color_swatches[0].set_color(QColor("#ff0000"))
        self.add_color_btn.setEnabled(True)
        self.status_label.setText("Reset to defaults")
        self.status_label.setStyleSheet("font-size: 11px; color: #888;")
        self._update_preview()


def main():
    app = QApplication(sys.argv)
    app.setApplicationName("QuadCast RGB")
    app.setDesktopFileName("quadcast-rgb-gui")
    window = MainWindow()
    window.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
