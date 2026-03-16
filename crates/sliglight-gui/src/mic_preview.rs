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

        // --- Mic body dimensions (larger, more prominent) ---
        let body_w = w * 0.55;
        let body_h = h * 0.62;
        let body_x = (w - body_w) / 2.0;
        let body_y = h * 0.06;
        let body_radius = body_w * 0.24;

        // --- Outer ambient glow (soft light spill onto background) ---
        let glow_color = Color {
            r: (self.upper_color.r + self.lower_color.r) / 2.0,
            g: (self.upper_color.g + self.lower_color.g) / 2.0,
            b: (self.upper_color.b + self.lower_color.b) / 2.0,
            a: 1.0,
        };
        let outer_glow_layers: &[(f32, f32)] = &[
            (24.0, 0.015),
            (18.0, 0.025),
            (12.0, 0.04),
            (8.0, 0.06),
            (4.0, 0.08),
        ];
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

        // --- Dark mic body background (behind LED fills) ---
        let body_bg = Path::rounded_rectangle(
            Point::new(body_x, body_y),
            Size::new(body_w, body_h),
            body_radius.into(),
        );
        frame.fill(&body_bg, Color::from_rgb8(0x11, 0x11, 0x1B));

        // --- Upper LED half ---
        let upper_h = body_h * 0.48;
        draw_led_fill(
            &mut frame,
            body_x,
            body_y,
            body_w,
            upper_h,
            body_radius,
            body_radius,
            0.0,
            0.0,
            self.upper_color,
        );

        // --- Lower LED half ---
        let lower_y = body_y + body_h * 0.52;
        let lower_h = body_h * 0.48;
        draw_led_fill(
            &mut frame,
            body_x,
            lower_y,
            body_w,
            lower_h,
            0.0,
            0.0,
            body_radius,
            body_radius,
            self.lower_color,
        );

        // --- Mesh grille (denser, cross-hatch pattern) ---
        let grille_x_start = body_x + body_w * 0.08;
        let grille_x_end = body_x + body_w * 0.92;
        let grille_top = body_y + body_h * 0.10;
        let grille_bot = body_y + body_h * 0.93;
        let num_lines = 32;
        let grille_dark = Color::from_rgba8(0x00, 0x00, 0x00, 0.14);
        let grille_light = Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.03);

        for i in 0..num_lines {
            let t = i as f32 / (num_lines - 1) as f32;
            let y = grille_top + t * (grille_bot - grille_top);
            // Dark line
            let line = Path::line(
                Point::new(grille_x_start, y),
                Point::new(grille_x_end, y),
            );
            frame.stroke(
                &line,
                Stroke::default()
                    .with_color(grille_dark)
                    .with_width(0.7),
            );
            // Light line offset (creates depth illusion)
            if i < num_lines - 1 {
                let yl = y + 1.0;
                let line_l = Path::line(
                    Point::new(grille_x_start, yl),
                    Point::new(grille_x_end, yl),
                );
                frame.stroke(
                    &line_l,
                    Stroke::default()
                        .with_color(grille_light)
                        .with_width(0.4),
                );
            }
        }

        // --- Divider line between zones ---
        let div_y = body_y + body_h * 0.50;
        let div_line = Path::line(
            Point::new(body_x + body_w * 0.04, div_y),
            Point::new(body_x + body_w * 0.96, div_y),
        );
        frame.stroke(
            &div_line,
            Stroke::default()
                .with_color(Color::from_rgba8(0x00, 0x00, 0x00, 0.90))
                .with_width(3.0),
        );
        // Highlight below divider
        let div_hl = Path::line(
            Point::new(body_x + body_w * 0.06, div_y + 1.5),
            Point::new(body_x + body_w * 0.94, div_y + 1.5),
        );
        frame.stroke(
            &div_hl,
            Stroke::default()
                .with_color(Color::from_rgba8(0xAA, 0xAA, 0xBB, 0.18))
                .with_width(0.8),
        );

        // --- Outer body border (crisp edge) ---
        let body_border = Path::rounded_rectangle(
            Point::new(body_x, body_y),
            Size::new(body_w, body_h),
            body_radius.into(),
        );
        frame.stroke(
            &body_border,
            Stroke::default()
                .with_color(Color::from_rgba8(0x66, 0x66, 0x77, 0.50))
                .with_width(1.5),
        );

        // --- Mute button (circle at top of mic) ---
        let mute_cx = w / 2.0;
        let mute_cy = body_y + body_h * 0.055;
        let mute_r = body_w * 0.065;
        let mute = Path::circle(Point::new(mute_cx, mute_cy), mute_r);
        frame.fill(&mute, Color::from_rgb8(0x22, 0x22, 0x30));
        frame.stroke(
            &mute,
            Stroke::default()
                .with_color(Color::from_rgba8(0x99, 0x99, 0xAA, 0.60))
                .with_width(1.0),
        );
        // Mute button highlight dot
        let mute_dot = Path::circle(
            Point::new(mute_cx, mute_cy - mute_r * 0.15),
            mute_r * 0.25,
        );
        frame.fill(&mute_dot, Color::from_rgba8(0xCC, 0xCC, 0xDD, 0.20));

        // --- Stand (tapered) ---
        let stand_w = body_w * 0.12;
        let stand_h = h * 0.16;
        let stand_x = (w - stand_w) / 2.0;
        let stand_y = body_y + body_h;
        let stand = Path::rounded_rectangle(
            Point::new(stand_x, stand_y),
            Size::new(stand_w, stand_h),
            (stand_w * 0.2).into(),
        );
        frame.fill(&stand, Color::from_rgb8(0x16, 0x16, 0x24));
        frame.stroke(
            &stand,
            Stroke::default()
                .with_color(Color::from_rgba8(0x44, 0x44, 0x55, 0.45))
                .with_width(1.0),
        );
        // Stand center highlight
        let stand_hl_w = stand_w * 0.3;
        let stand_hl = Path::rounded_rectangle(
            Point::new((w - stand_hl_w) / 2.0, stand_y + stand_h * 0.1),
            Size::new(stand_hl_w, stand_h * 0.8),
            (stand_hl_w * 0.3).into(),
        );
        frame.fill(&stand_hl, Color::from_rgba8(0x88, 0x88, 0x99, 0.08));

        // --- Base (wider, with subtle gradient) ---
        let base_w = body_w * 0.75;
        let base_h = h * 0.045;
        let base_x = (w - base_w) / 2.0;
        let base_y = stand_y + stand_h - base_h * 0.25;
        let base_radius = base_h * 0.45;
        let base = Path::rounded_rectangle(
            Point::new(base_x, base_y),
            Size::new(base_w, base_h),
            base_radius.into(),
        );
        frame.fill(&base, Color::from_rgb8(0x1A, 0x1A, 0x28));
        frame.stroke(
            &base,
            Stroke::default()
                .with_color(Color::from_rgba8(0x55, 0x55, 0x66, 0.45))
                .with_width(1.0),
        );
        // Base top highlight
        let base_hl = Path::line(
            Point::new(base_x + base_w * 0.15, base_y + 1.0),
            Point::new(base_x + base_w * 0.85, base_y + 1.0),
        );
        frame.stroke(
            &base_hl,
            Stroke::default()
                .with_color(Color::from_rgba8(0xAA, 0xAA, 0xBB, 0.10))
                .with_width(0.8),
        );

        vec![frame.into_geometry()]
    }
}

/// Fill a half of the mic body with an LED color and glow effect.
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
    // Inner glow layers — drawn from outermost inward
    let glow_layers: &[(f32, f32)] = &[
        (8.0, 0.04),
        (5.0, 0.08),
        (3.0, 0.14),
        (1.5, 0.22),
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

    // Top specular highlight
    let hl_h = h * 0.10;
    let highlight = Path::rounded_rectangle(
        Point::new(x + w * 0.08, y + h * 0.04),
        Size::new(w * 0.84, hl_h),
        (tl.max(tr) * 0.4).into(),
    );
    let hl_color = Color {
        r: (color.r + 0.35).min(1.0),
        g: (color.g + 0.35).min(1.0),
        b: (color.b + 0.35).min(1.0),
        a: 0.18,
    };
    frame.fill(&highlight, hl_color);

    // Bottom edge shadow
    let shadow_h = h * 0.06;
    let shadow = Path::rounded_rectangle(
        Point::new(x + w * 0.05, y + h - shadow_h),
        Size::new(w * 0.90, shadow_h),
        (bl.max(br) * 0.4).into(),
    );
    frame.fill(
        &shadow,
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.10,
        },
    );
}
