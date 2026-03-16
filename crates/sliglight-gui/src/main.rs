//! Sliglight — iced GUI for HyperX QuadCast RGB control.

mod engine;
mod mic_preview;

use iced::widget::{button, canvas::Canvas, column, container, row, slider, text, Space};
use iced::{Background, Border, Color, Element, Length, Subscription, Task, Theme};

use mic_preview::MicPreview;

use sliglight_core::animations::{Mode, Zone};

// ---------------------------------------------------------------------------
// Palette — Catppuccin Mocha accents used for custom styling
// ---------------------------------------------------------------------------

const SURFACE0: Color = Color::from_rgb(0x31 as f32 / 255.0, 0x32 as f32 / 255.0, 0x47 as f32 / 255.0);
const SURFACE1: Color = Color::from_rgb(0x45 as f32 / 255.0, 0x47 as f32 / 255.0, 0x5A as f32 / 255.0);
const SURFACE2: Color = Color::from_rgb(0x58 as f32 / 255.0, 0x5B as f32 / 255.0, 0x70 as f32 / 255.0);
const OVERLAY0: Color = Color::from_rgb(0x6C as f32 / 255.0, 0x70 as f32 / 255.0, 0x86 as f32 / 255.0);
const SUBTEXT0: Color = Color::from_rgb(0xA6 as f32 / 255.0, 0xAD as f32 / 255.0, 0xC8 as f32 / 255.0);
const TEXT: Color = Color::from_rgb(0xCD as f32 / 255.0, 0xD6 as f32 / 255.0, 0xF4 as f32 / 255.0);
const LAVENDER: Color = Color::from_rgb(0xB4 as f32 / 255.0, 0xBE as f32 / 255.0, 0xFE as f32 / 255.0);
const GREEN: Color = Color::from_rgb(0xA6 as f32 / 255.0, 0xE3 as f32 / 255.0, 0xA1 as f32 / 255.0);
const RED: Color = Color::from_rgb(0xF3 as f32 / 255.0, 0x8B as f32 / 255.0, 0xA8 as f32 / 255.0);
const BASE: Color = Color::from_rgb(0x1E as f32 / 255.0, 0x1E as f32 / 255.0, 0x2E as f32 / 255.0);
const MANTLE: Color = Color::from_rgb(0x18 as f32 / 255.0, 0x18 as f32 / 255.0, 0x25 as f32 / 255.0);

/// Default LED preview color (matches unlit mic body).
const DEFAULT_PREVIEW: Color = SURFACE0;

fn main() -> iced::Result {
    env_logger::init();
    iced::application(boot, update, view)
        .title("Sliglight")
        .theme(Theme::CatppuccinMocha)
        .subscription(subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(860.0, 680.0),
            icon: load_icon(),
            ..Default::default()
        })
        .run()
}

fn load_icon() -> Option<iced::window::icon::Icon> {
    let svg_data = include_bytes!("../../../resources/sliglight.svg");
    let tree = resvg::usvg::Tree::from_data(svg_data, &resvg::usvg::Options::default()).ok()?;
    let size = 64u32;
    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    let svg_size = tree.size();
    let scale = size as f32 / svg_size.width().max(svg_size.height());
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    iced::window::icon::from_rgba(pixmap.data().to_vec(), size, size).ok()
}

struct App {
    zone: Zone,
    mode: Mode,
    brightness: u8,
    speed: u8,
    colors: Vec<(u8, u8, u8)>,
    editing_color: Option<usize>,
    status: Status,
    engine: Option<engine::Handle>,
    upper_preview_color: Color,
    lower_preview_color: Color,
}

enum Status {
    Idle,
    Connected,
    Error(String),
}

#[derive(Debug, Clone)]
enum Message {
    SetZone(Zone),
    SetMode(Mode),
    SetBrightness(u8),
    SetSpeed(u8),
    AddColor,
    RemoveColor(usize),
    #[allow(dead_code)]
    SetColor(usize, (u8, u8, u8)),
    ToggleColorEditor(usize),
    SetColorR(u8),
    SetColorG(u8),
    SetColorB(u8),
    Apply,
    Reset,
    EngineEvent(engine::Event),
}

fn boot() -> (App, Task<Message>) {
    (
        App {
            zone: Zone::Both,
            mode: Mode::Solid,
            brightness: 80,
            speed: 81,
            colors: vec![(255, 0, 0)],
            editing_color: None,
            status: Status::Idle,
            engine: None,
            upper_preview_color: DEFAULT_PREVIEW,
            lower_preview_color: DEFAULT_PREVIEW,
        },
        Task::none(),
    )
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::SetZone(z) => app.zone = z,
        Message::SetMode(m) => app.mode = m,
        Message::SetBrightness(b) => app.brightness = b,
        Message::SetSpeed(s) => app.speed = s,
        Message::AddColor => {
            if app.colors.len() < 11 {
                let cycle = [
                    (0, 255, 0),
                    (0, 0, 255),
                    (255, 255, 0),
                    (0, 255, 255),
                    (255, 0, 255),
                    (255, 136, 0),
                    (136, 255, 0),
                    (255, 0, 136),
                    (0, 136, 255),
                    (136, 0, 255),
                ];
                let c = cycle[(app.colors.len() - 1) % cycle.len()];
                app.colors.push(c);
            }
        }
        Message::RemoveColor(i) => {
            if app.colors.len() > 1 && i < app.colors.len() {
                app.colors.remove(i);
                if let Some(idx) = app.editing_color {
                    if idx == i {
                        app.editing_color = None;
                    } else if idx > i {
                        app.editing_color = Some(idx - 1);
                    }
                }
            }
        }
        Message::SetColor(i, c) => {
            if i < app.colors.len() {
                app.colors[i] = c;
            }
        }
        Message::ToggleColorEditor(i) => {
            if app.editing_color == Some(i) {
                app.editing_color = None;
            } else if i < app.colors.len() {
                app.editing_color = Some(i);
            }
        }
        Message::SetColorR(v) => {
            if let Some(idx) = app.editing_color {
                if idx < app.colors.len() {
                    app.colors[idx].0 = v;
                }
            }
        }
        Message::SetColorG(v) => {
            if let Some(idx) = app.editing_color {
                if idx < app.colors.len() {
                    app.colors[idx].1 = v;
                }
            }
        }
        Message::SetColorB(v) => {
            if let Some(idx) = app.editing_color {
                if idx < app.colors.len() {
                    app.colors[idx].2 = v;
                }
            }
        }
        Message::Apply => {
            app.editing_color = None;
            app.engine = Some(engine::Handle::start(
                app.mode,
                app.colors.clone(),
                app.brightness,
                app.speed,
                app.zone,
            ));
            app.status = Status::Idle;
        }
        Message::Reset => {
            app.zone = Zone::Both;
            app.mode = Mode::Solid;
            app.brightness = 80;
            app.speed = 81;
            app.colors = vec![(255, 0, 0)];
            app.editing_color = None;
            app.engine = None;
            app.status = Status::Idle;
            app.upper_preview_color = DEFAULT_PREVIEW;
            app.lower_preview_color = DEFAULT_PREVIEW;
        }
        Message::EngineEvent(e) => match e {
            engine::Event::Connected => app.status = Status::Connected,
            engine::Event::Error(msg) => app.status = Status::Error(msg),
            engine::Event::FrameSent { upper, lower } => {
                app.upper_preview_color = Color::from_rgb8(upper.0, upper.1, upper.2);
                app.lower_preview_color = Color::from_rgb8(lower.0, lower.1, lower.2);
            }
        },
    }
    Task::none()
}

fn subscription(app: &App) -> Subscription<Message> {
    if let Some(handle) = &app.engine {
        handle.subscription().map(Message::EngineEvent)
    } else {
        Subscription::none()
    }
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

fn view(app: &App) -> Element<'_, Message> {
    let mic = container(
        Canvas::new(MicPreview {
            upper_color: app.upper_preview_color,
            lower_color: app.lower_preview_color,
        })
        .width(180)
        .height(360),
    )
    .padding([32, 20])
    .center_y(Length::Fill);

    let controls = column![
        view_header(),
        view_zone_selector(app),
        view_mode_grid(app),
        view_sliders(app),
        view_color_palette(app),
        Space::new().height(8),
        view_actions(app),
        Space::new().height(Length::Fill),
        view_status(app),
    ]
    .spacing(14)
    .padding([28, 32])
    .width(Length::Fill);

    // Vertical separator between mic and controls
    let separator = container(
        container(Space::new().width(1).height(Length::Fill))
            .style(|_theme: &Theme| container::Style {
                background: Some(Background::Color(SURFACE0)),
                ..container::Style::default()
            })
    )
    .padding([40, 0])
    .height(Length::Fill);

    container(row![mic, separator, controls])
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(BASE)),
            ..container::Style::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// View helpers
// ---------------------------------------------------------------------------

fn section_label(label: &str) -> Element<'_, Message> {
    text(label.to_uppercase())
        .size(11)
        .color(OVERLAY0)
        .into()
}

fn view_header() -> Element<'static, Message> {
    column![
        text("Sliglight").size(24).color(TEXT),
        text("QuadCast 2S RGB Control").size(12).color(SURFACE2),
    ]
    .spacing(2)
    .into()
}

fn view_zone_selector(app: &App) -> Element<'_, Message> {
    let zones = [
        (Zone::Both, "Both"),
        (Zone::Upper, "Upper"),
        (Zone::Lower, "Lower"),
    ];
    let buttons: Vec<Element<'_, Message>> = zones
        .iter()
        .map(|(zone, label)| {
            let selected = app.zone == *zone;
            let z = *zone;
            button(text(*label).size(13).center())
                .width(Length::Fill)
                .height(32)
                .on_press(Message::SetZone(z))
                .style(move |theme, status| pill_btn_style(theme, status, selected))
                .into()
        })
        .collect();

    column![
        section_label("Zone"),
        row(buttons).spacing(6).width(Length::Fill),
    ]
    .spacing(6)
    .into()
}

fn view_mode_grid(app: &App) -> Element<'_, Message> {
    let modes = Mode::ALL;
    let top_row: Vec<Element<'_, Message>> = modes[..3]
        .iter()
        .map(|m| mode_button(app, *m))
        .collect();
    let bottom_row: Vec<Element<'_, Message>> = modes[3..]
        .iter()
        .map(|m| mode_button(app, *m))
        .collect();

    column![
        section_label("Mode"),
        row(top_row).spacing(6).width(Length::Fill),
        row(bottom_row).spacing(6).width(Length::Fill),
    ]
    .spacing(6)
    .into()
}

fn mode_button<'a>(app: &App, mode: Mode) -> Element<'a, Message> {
    let selected = app.mode == mode;
    // Capitalize first letter, no emoji
    let name = mode.name();
    let label = format!("{}{}", &name[..1].to_uppercase(), &name[1..]);
    button(text(label).size(13).center())
        .width(Length::Fill)
        .height(32)
        .on_press(Message::SetMode(mode))
        .style(move |theme, status| pill_btn_style(theme, status, selected))
        .into()
}

fn view_sliders(app: &App) -> Element<'_, Message> {
    column![
        section_label("Settings"),
        row![
            text("Brightness").size(12).color(SUBTEXT0).width(Length::Fill),
            text(format!("{}", app.brightness)).size(12).color(TEXT),
        ],
        slider(0..=100, app.brightness, Message::SetBrightness),
        row![
            text("Speed").size(12).color(SUBTEXT0).width(Length::Fill),
            text(format!("{}", app.speed)).size(12).color(TEXT),
        ],
        slider(0..=100, app.speed, Message::SetSpeed),
    ]
    .spacing(4)
    .into()
}

fn view_color_palette(app: &App) -> Element<'_, Message> {
    // --- Color Strip: circular swatches in a uniform row ---
    let mut swatches: Vec<Element<'_, Message>> = Vec::new();

    for (i, &(r, g, b)) in app.colors.iter().enumerate() {
        let is_selected = app.editing_color == Some(i);
        swatches.push(color_swatch(i, r, g, b, is_selected));
    }

    // Add button — same size circle, dashed-border feel
    if app.colors.len() < 11 {
        let add_btn = button(
            container(text("+").size(14).center().color(OVERLAY0))
                .width(36)
                .height(36)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .padding(0)
        .on_press(Message::AddColor)
        .style(|_theme, status| {
            let border_col = if status == button::Status::Hovered { LAVENDER } else { SURFACE1 };
            button::Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border: Border::default().rounded(20).width(1.5).color(border_col),
                ..button::Style::default()
            }
        });
        // Wrap in same-size container as swatches for alignment
        swatches.push(
            container(add_btn)
                .width(42)
                .height(42)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
        );
    }

    let strip = row(swatches)
        .spacing(8)
        .align_y(iced::Alignment::Center);

    let mut palette = column![section_label("Colors"), strip].spacing(8);

    // --- Control Deck: appears below strip when a color is selected ---
    if let Some(idx) = app.editing_color {
        if idx < app.colors.len() {
            let can_remove = app.colors.len() > 1 && idx > 0;
            palette = palette.push(view_color_deck(app.colors[idx], idx, can_remove));
        }
    }

    palette.into()
}

/// A single circular color swatch with glow ring when selected.
fn color_swatch(index: usize, r: u8, g: u8, b: u8, selected: bool) -> Element<'static, Message> {
    let color = Color::from_rgb8(r, g, b);

    // Inner circle — the actual color
    let inner = container(Space::new().width(36).height(36))
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(color)),
            border: Border::default().rounded(18),
            ..container::Style::default()
        });

    // Outer wrapper — adds glow ring when selected
    let outer_size = 42;
    let swatch_container = if selected {
        // Glow ring: accent-colored container slightly larger than the swatch
        container(
            container(inner)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(outer_size)
        .height(outer_size)
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(LAVENDER)),
            border: Border::default().rounded(21),
            ..container::Style::default()
        })
    } else {
        container(
            container(inner)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(outer_size)
        .height(outer_size)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: Border::default().rounded(21),
            ..container::Style::default()
        })
    };

    button(swatch_container)
        .padding(0)
        .on_press(Message::ToggleColorEditor(index))
        .style(|_theme, _status| button::Style {
            background: None,
            border: Border::default(),
            ..button::Style::default()
        })
        .into()
}

/// The Control Deck — contextual editor panel below the color strip.
fn view_color_deck(
    (r, g, b): (u8, u8, u8),
    _index: usize,
    can_remove: bool,
) -> Element<'static, Message> {
    // Large color preview circle
    let preview = container(Space::new().width(56).height(56))
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(r, g, b))),
            border: Border::default().rounded(28),
            ..container::Style::default()
        });

    let hex_code = text(format!("#{r:02X}{g:02X}{b:02X}"))
        .size(12)
        .color(SUBTEXT0);

    let preview_col = column![preview, hex_code]
        .spacing(6)
        .align_x(iced::Alignment::Center);

    // Channel sliders with value readout
    let r_slider = channel_slider("R", r, RED, Message::SetColorR);
    let g_slider = channel_slider("G", g, GREEN, Message::SetColorG);
    let b_slider = channel_slider("B", b, LAVENDER, Message::SetColorB);

    let sliders = column![r_slider, g_slider, b_slider]
        .spacing(6)
        .width(Length::Fill);

    // Bottom row: remove button (right-aligned, only if removable)
    let bottom: Element<'_, Message> = if can_remove {
        container(
            button(text("Remove").size(11).center().color(RED))
                .padding([4, 14])
                .on_press(Message::RemoveColor(_index))
                .style(|_theme: &Theme, status| {
                    let bg = if status == button::Status::Hovered {
                        Color { a: 0.15, ..RED }
                    } else {
                        Color::TRANSPARENT
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        border: Border::default().rounded(6).width(1.0).color(
                            Color { a: 0.3, ..RED },
                        ),
                        ..button::Style::default()
                    }
                }),
        )
        .width(Length::Fill)
        .align_x(iced::Alignment::End)
        .into()
    } else {
        Space::new().height(0).into()
    };

    let content = column![
        row![preview_col, sliders]
            .spacing(16)
            .align_y(iced::Alignment::Center),
        bottom,
    ]
    .spacing(8);

    container(content)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(MANTLE)),
            border: Border::default().rounded(12).width(1.0).color(SURFACE0),
            ..container::Style::default()
        })
        .padding([14, 16])
        .width(Length::Fill)
        .into()
}

/// A single RGB channel slider row: label, slider, value.
fn channel_slider<'a>(
    label: &'a str,
    value: u8,
    color: Color,
    on_change: fn(u8) -> Message,
) -> Element<'a, Message> {
    row![
        text(label).size(12).color(color).width(16),
        slider(0..=255, value, on_change).width(Length::Fill),
        text(format!("{value:>3}")).size(11).color(SUBTEXT0).width(28),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn view_actions(_app: &App) -> Element<'_, Message> {
    row![
        button(text("Apply").size(13).center())
            .width(Length::Fill)
            .height(36)
            .on_press(Message::Apply)
            .style(|_theme, status| apply_btn_style(status)),
        button(text("Reset").size(13).center().color(SUBTEXT0))
            .width(Length::Fill)
            .height(36)
            .on_press(Message::Reset)
            .style(|_theme, status| {
                let bg = if status == button::Status::Hovered { SURFACE1 } else { SURFACE0 };
                button::Style {
                    background: Some(Background::Color(bg)),
                    border: Border::default().rounded(8),
                    ..button::Style::default()
                }
            }),
    ]
    .spacing(10)
    .into()
}

fn view_status(app: &App) -> Element<'_, Message> {
    let (label, dot_color) = match &app.status {
        Status::Idle => ("Ready", SURFACE2),
        Status::Connected => ("Connected", GREEN),
        Status::Error(_) => ("Error", RED),
    };
    let status_text = match &app.status {
        Status::Error(e) => format!("{e}"),
        _ => label.to_string(),
    };

    // Status indicator: colored dot + text
    let dot = container(Space::new().width(6).height(6))
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(dot_color)),
            border: Border::default().rounded(3),
            ..container::Style::default()
        });

    container(
        row![dot, text(status_text).size(12).color(SUBTEXT0)]
            .spacing(8)
            .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .into()
}

// ---------------------------------------------------------------------------
// Button styles
// ---------------------------------------------------------------------------

fn pill_btn_style(
    _theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    if selected {
        button::Style {
            background: Some(Background::Color(LAVENDER)),
            text_color: BASE,
            border: Border::default().rounded(8),
            ..button::Style::default()
        }
    } else {
        let bg = if status == button::Status::Hovered {
            SURFACE1
        } else {
            SURFACE0
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: SUBTEXT0,
            border: Border::default().rounded(8),
            ..button::Style::default()
        }
    }
}


fn apply_btn_style(status: button::Status) -> button::Style {
    let bg = if status == button::Status::Hovered {
        Color::from_rgb(0.58, 0.82, 0.56)
    } else {
        GREEN
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: MANTLE,
        border: Border::default().rounded(8),
        ..button::Style::default()
    }
}
