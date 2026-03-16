//! Canvas widget that renders a stylised microphone with LED glow zones.
//!
//! The two LED zones reflect the current upper/lower colours being sent to the
//! QuadCast 2S. A soft glow is drawn around each zone by layering translucent
//! copies at increasing sizes.

use iced::mouse;
use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

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

        // All dimensions expressed as fractions of the canvas so the drawing
        // scales with the widget size.

        // --- Mic body (tall rounded rectangle) ---
        let body_w = w * 0.50;
        let body_h = h * 0.60;
        let body_x = (w - body_w) / 2.0;
        let body_y = h * 0.08;
        let body_radius = body_w * 0.22;

        let body = Path::rounded_rectangle(
            Point::new(body_x, body_y),
            Size::new(body_w, body_h),
            body_radius.into(),
        );
        frame.fill(&body, Color::from_rgb8(0x14, 0x14, 0x20));

        // Thin border around the body
        frame.stroke(
            &body,
            Stroke::default()
                .with_color(Color::from_rgba8(0x55, 0x55, 0x66, 0.67))
                .with_width(1.5),
        );

        // --- Mute button (small ellipse at top of mic) ---
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

        // --- LED zones ---
        let led_w = body_w * 0.72;
        let led_h = body_h * 0.28;
        let led_x = body_x + (body_w - led_w) / 2.0;
        let led_radius = led_w * 0.12;

        // Upper LED zone
        let upper_y = body_y + body_h * 0.16;
        draw_led_zone(&mut frame, led_x, upper_y, led_w, led_h, led_radius, self.upper_color);

        // Lower LED zone
        let lower_y = body_y + body_h * 0.56;
        draw_led_zone(&mut frame, led_x, lower_y, led_w, led_h, led_radius, self.lower_color);

        // --- Mesh grille lines (subtle horizontal texture) ---
        let grille_x_start = body_x + body_w * 0.12;
        let grille_x_end = body_x + body_w * 0.88;
        let grille_top = body_y + body_h * 0.12;
        let grille_bot = body_y + body_h * 0.92;
        let num_lines = 18;
        let grille_color = Color::from_rgba8(0x40, 0x40, 0x50, 0.27);

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

/// Draw an LED zone rectangle with a soft glow halo.
///
/// The glow is produced by rendering progressively larger, more transparent
/// copies of the rectangle behind the solid fill.
fn draw_led_zone(
    frame: &mut Frame,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: Color,
) {
    // Glow layers — drawn from outermost (largest, most transparent) inwards.
    let glow_layers: &[(f32, f32)] = &[
        (8.0, 0.06),
        (6.0, 0.10),
        (4.0, 0.16),
        (2.0, 0.24),
    ];

    for &(expand, alpha) in glow_layers {
        let glow_color = Color {
            a: color.a * alpha,
            ..color
        };
        let glow = Path::rounded_rectangle(
            Point::new(x - expand, y - expand),
            Size::new(w + expand * 2.0, h + expand * 2.0),
            (radius + expand * 0.5).into(),
        );
        frame.fill(&glow, glow_color);
    }

    // Solid LED fill
    let led = Path::rounded_rectangle(
        Point::new(x, y),
        Size::new(w, h),
        radius.into(),
    );
    frame.fill(&led, color);

    // Subtle inner highlight at the top of the LED zone
    let highlight_h = h * 0.15;
    let highlight = Path::rounded_rectangle(
        Point::new(x + w * 0.05, y + h * 0.04),
        Size::new(w * 0.90, highlight_h),
        (radius * 0.6).into(),
    );
    let highlight_color = Color {
        r: (color.r + 0.3).min(1.0),
        g: (color.g + 0.3).min(1.0),
        b: (color.b + 0.3).min(1.0),
        a: 0.18,
    };
    frame.fill(&highlight, highlight_color);
}
