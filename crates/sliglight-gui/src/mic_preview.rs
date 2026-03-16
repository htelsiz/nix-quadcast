//! Canvas widget that renders a stylised microphone with LED glow zones.
//!
//! The mic body is divided into upper and lower LED halves by a visible
//! separator, matching the real QuadCast 2S layout where 54 LEDs wrap each
//! half of the mesh grille.

use iced::mouse;
use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::{border, Color, Point, Rectangle, Renderer, Size, Theme};

/// Microphone preview state passed in from the application each frame.
pub struct MicPreview {
    pub upper_color: Color,
    pub lower_color: Color,
}

impl<Message> canvas::Program<Message> for MicPreview {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;

        // --- Mic body dimensions ---
        let body_w = w * 0.50;
        let body_h = h * 0.60;
        let body_x = (w - body_w) / 2.0;
        let body_y = h * 0.08;
        let body_radius = body_w * 0.22;

        // --- Outer body glow ---
        let glow_color = Color {
            r: (self.upper_color.r + self.lower_color.r) / 2.0,
            g: (self.upper_color.g + self.lower_color.g) / 2.0,
            b: (self.upper_color.b + self.lower_color.b) / 2.0,
            a: 1.0,
        };
        let outer_glow_layers: &[(f32, f32)] = &[(14.0, 0.03), (10.0, 0.05), (6.0, 0.08)];
        for &(expand, alpha) in outer_glow_layers {
            let gc = Color {
                a: alpha,
                ..glow_color
            };
            let glow = Path::rounded_rectangle(
                Point::new(body_x - expand, body_y - expand),
                Size::new(body_w + expand * 2.0, body_h + expand * 2.0),
                (body_radius + expand * 0.5).into(),
            );
            frame.fill(&glow, gc);
        }

        // --- Upper LED half (top half of mic body) ---
        let upper_h = body_h * 0.48;
        draw_led_fill(
            &mut frame,
            body_x,
            body_y,
            body_w,
            upper_h,
            body_radius,
            body_radius, // top corners rounded
            0.0,         // bottom corners square (meets divider)
            0.0,
            self.upper_color,
        );

        // --- Lower LED half (bottom half of mic body) ---
        let lower_y = body_y + body_h * 0.52;
        let lower_h = body_h * 0.48;
        draw_led_fill(
            &mut frame,
            body_x,
            lower_y,
            body_w,
            lower_h,
            0.0, // top corners square (meets divider)
            0.0,
            body_radius, // bottom corners rounded
            body_radius,
            self.lower_color,
        );

        // --- Divider line between zones ---
        let div_y = body_y + body_h * 0.50;
        let div_line = Path::line(
            Point::new(body_x + body_w * 0.06, div_y),
            Point::new(body_x + body_w * 0.94, div_y),
        );
        frame.stroke(
            &div_line,
            Stroke::default()
                .with_color(Color::from_rgba8(0x00, 0x00, 0x00, 0.85))
                .with_width(2.5),
        );
        // Subtle highlight below divider
        let div_highlight = Path::line(
            Point::new(body_x + body_w * 0.06, div_y + 1.5),
            Point::new(body_x + body_w * 0.94, div_y + 1.5),
        );
        frame.stroke(
            &div_highlight,
            Stroke::default()
                .with_color(Color::from_rgba8(0x88, 0x88, 0x99, 0.25))
                .with_width(1.0),
        );

        // --- Outer body border ---
        let body = Path::rounded_rectangle(
            Point::new(body_x, body_y),
            Size::new(body_w, body_h),
            body_radius.into(),
        );
        frame.stroke(
            &body,
            Stroke::default()
                .with_color(Color::from_rgba8(0x55, 0x55, 0x66, 0.67))
                .with_width(1.5),
        );

        // --- Mute button (small circle at top of mic) ---
        let mute_cx = w / 2.0;
        let mute_cy = body_y + body_h * 0.06;
        let mute_r = body_w * 0.07;
        let mute = Path::circle(Point::new(mute_cx, mute_cy), mute_r);
        frame.fill(&mute, Color::from_rgb8(0x2A, 0x2A, 0x38));
        frame.stroke(
            &mute,
            Stroke::default()
                .with_color(Color::from_rgba8(0x88, 0x88, 0x99, 0.80))
                .with_width(1.0),
        );

        // --- Mesh grille lines (subtle horizontal texture over LED zones) ---
        let grille_x_start = body_x + body_w * 0.10;
        let grille_x_end = body_x + body_w * 0.90;
        let grille_top = body_y + body_h * 0.12;
        let grille_bot = body_y + body_h * 0.92;
        let num_lines = 20;
        let grille_color = Color::from_rgba8(0x00, 0x00, 0x00, 0.18);

        for i in 0..num_lines {
            let t = i as f32 / (num_lines - 1) as f32;
            let y = grille_top + t * (grille_bot - grille_top);
            let line = Path::line(
                Point::new(grille_x_start, y),
                Point::new(grille_x_end, y),
            );
            frame.stroke(
                &line,
                Stroke::default()
                    .with_color(grille_color)
                    .with_width(0.8),
            );
        }

        // --- Stand (thin rectangle below mic body) ---
        let stand_w = body_w * 0.14;
        let stand_h = h * 0.16;
        let stand_x = (w - stand_w) / 2.0;
        let stand_y = body_y + body_h;

        let stand = Path::rounded_rectangle(
            Point::new(stand_x, stand_y),
            Size::new(stand_w, stand_h),
            (stand_w * 0.15).into(),
        );
        frame.fill(&stand, Color::from_rgb8(0x1A, 0x1A, 0x28));
        frame.stroke(
            &stand,
            Stroke::default()
                .with_color(Color::from_rgba8(0x44, 0x44, 0x55, 0.53))
                .with_width(1.0),
        );

        // --- Base (wider rectangle at the bottom) ---
        let base_w = body_w * 0.80;
        let base_h = h * 0.05;
        let base_x = (w - base_w) / 2.0;
        let base_y = stand_y + stand_h - base_h * 0.3;
        let base_radius = base_h * 0.40;

        let base = Path::rounded_rectangle(
            Point::new(base_x, base_y),
            Size::new(base_w, base_h),
            base_radius.into(),
        );
        frame.fill(&base, Color::from_rgb8(0x1E, 0x1E, 0x2C));
        frame.stroke(
            &base,
            Stroke::default()
                .with_color(Color::from_rgba8(0x55, 0x55, 0x66, 0.53))
                .with_width(1.0),
        );

        vec![frame.into_geometry()]
    }
}

/// Fill a half of the mic body with an LED color and glow effect.
///
/// Uses per-corner radii so the top half has rounded top corners and flat
/// bottom corners (and vice versa for the lower half).
#[allow(clippy::too_many_arguments)]
fn draw_led_fill(
    frame: &mut Frame,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    tl: f32,
    tr: f32,
    bl: f32,
    br: f32,
    color: Color,
) {
    // Glow layers — drawn from outermost inward
    let glow_layers: &[(f32, f32)] = &[
        (6.0, 0.05),
        (4.0, 0.10),
        (2.0, 0.18),
    ];

    for &(expand, alpha) in glow_layers {
        let glow_color = Color {
            a: color.a * alpha,
            ..color
        };
        let glow = Path::rounded_rectangle(
            Point::new(x - expand, y - expand),
            Size::new(w + expand * 2.0, h + expand * 2.0),
            border::Radius {
                top_left: tl + expand * 0.5,
                top_right: tr + expand * 0.5,
                bottom_left: bl + expand * 0.5,
                bottom_right: br + expand * 0.5,
            }
            .into(),
        );
        frame.fill(&glow, glow_color);
    }

    // Solid LED fill
    let led = Path::rounded_rectangle(
        Point::new(x, y),
        Size::new(w, h),
        border::Radius {
            top_left: tl,
            top_right: tr,
            bottom_left: bl,
            bottom_right: br,
        }
        .into(),
    );
    frame.fill(&led, color);

    // Subtle top highlight
    let hl_h = h * 0.12;
    let highlight = Path::rounded_rectangle(
        Point::new(x + w * 0.05, y + h * 0.03),
        Size::new(w * 0.90, hl_h),
        (tl.max(tr) * 0.5).into(),
    );
    let highlight_color = Color {
        r: (color.r + 0.3).min(1.0),
        g: (color.g + 0.3).min(1.0),
        b: (color.b + 0.3).min(1.0),
        a: 0.15,
    };
    frame.fill(&highlight, highlight_color);
}
