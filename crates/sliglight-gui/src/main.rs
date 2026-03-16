//! Sliglight — iced GUI for HyperX QuadCast RGB control.

mod engine;
mod mic_preview;

use iced::widget::{button, canvas::Canvas, column, container, row, slider, text, text_input, Space};
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
    hex_input: String,
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
    HexInputChanged(String),
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
            hex_input: String::new(),
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
                let (r, g, b) = app.colors[i];
                app.hex_input = format!("{r:02X}{g:02X}{b:02X}");
            }
        }
        Message::HexInputChanged(s) => {
            let cleaned: String = s
                .chars()
                .filter(|c| c.is_ascii_hexdigit())
                .take(6)
                .map(|c| c.to_ascii_uppercase())
                .collect();
            app.hex_input = cleaned;
            if app.hex_input.len() == 6 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&app.hex_input[0..2], 16),
                    u8::from_str_radix(&app.hex_input[2..4], 16),
                    u8::from_str_radix(&app.hex_input[4..6], 16),
                ) {
                    if let Some(idx) = app.editing_color {
                        if idx < app.colors.len() {
                            app.colors[idx] = (r, g, b);
                        }
                    }
                }
            }
        }
        Message::SetColorR(v) => {
            if let Some(idx) = app.editing_color {
                if idx < app.colors.len() {
                    app.colors[idx].0 = v;
                    let (r, g, b) = app.colors[idx];
                    app.hex_input = format!("{r:02X}{g:02X}{b:02X}");
                }
            }
        }
        Message::SetColorG(v) => {
            if let Some(idx) = app.editing_color {
                if idx < app.colors.len() {
                    app.colors[idx].1 = v;
                    let (r, g, b) = app.colors[idx];
                    app.hex_input = format!("{r:02X}{g:02X}{b:02X}");
                }
            }
        }
        Message::SetColorB(v) => {
            if let Some(idx) = app.editing_color {
                if idx < app.colors.len() {
                    app.colors[idx].2 = v;
                    let (r, g, b) = app.colors[idx];
                    app.hex_input = format!("{r:02X}{g:02X}{b:02X}");
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
            app.hex_input = String::new();
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
    // Color chips: uniform rounded squares
    let mut chips: Vec<Element<'_, Message>> = app
        .colors
        .iter()
        .enumerate()
        .map(|(i, &(r, g, b))| color_chip(i, r, g, b, app.editing_color == Some(i)))
        .collect();

    if app.colors.len() < 11 {
        chips.push(add_chip());
    }

    let chip_row = row(chips).spacing(6).align_y(iced::Alignment::Center);
    let mut content = column![section_label("Colors"), chip_row].spacing(8);

    // Editor card appears below when a color is selected
    if let Some(idx) = app.editing_color {
        if idx < app.colors.len() {
            content = content.push(view_color_editor(app, idx));
        }
    }

    content.into()
}

/// Uniform rounded-square color chip.
fn color_chip(index: usize, r: u8, g: u8, b: u8, selected: bool) -> Element<'static, Message> {
    let color = Color::from_rgb8(r, g, b);
    let chip = container(Space::new().width(32).height(32)).style(move |_theme: &Theme| {
        let border = if selected {
            Border::default().rounded(6).width(2.0).color(LAVENDER)
        } else {
            Border::default().rounded(6).width(1.0).color(SURFACE1)
        };
        container::Style {
            background: Some(Background::Color(color)),
            border,
            ..container::Style::default()
        }
    });

    button(chip)
        .padding(0)
        .on_press(Message::ToggleColorEditor(index))
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            border: Border::default(),
            ..button::Style::default()
        })
        .into()
}

/// "+" chip matching swatch size.
fn add_chip() -> Element<'static, Message> {
    button(
        container(text("+").size(14).center().color(OVERLAY0))
            .width(32)
            .height(32)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .padding(0)
    .on_press(Message::AddColor)
    .style(|_theme: &Theme, status| {
        let border_col = if status == button::Status::Hovered {
            LAVENDER
        } else {
            SURFACE1
        };
        button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: Border::default().rounded(6).width(1.0).color(border_col),
            ..button::Style::default()
        }
    })
    .into()
}

/// Color editor card with hex input, RGB sliders, and remove button.
fn view_color_editor(app: &App, idx: usize) -> Element<'_, Message> {
    let (r, g, b) = app.colors[idx];
    let can_remove = app.colors.len() > 1;

    // Color preview swatch
    let preview = container(Space::new().width(40).height(40)).style(
        move |_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(r, g, b))),
            border: Border::default().rounded(8),
            ..container::Style::default()
        },
    );

    // Hex input: "#" label + text field
    let hex_field = text_input("000000", &app.hex_input)
        .on_input(Message::HexInputChanged)
        .size(14)
        .width(80)
        .style(|_theme: &Theme, status| {
            let border_col = match status {
                text_input::Status::Focused { .. } => LAVENDER,
                text_input::Status::Hovered => SURFACE2,
                _ => SURFACE1,
            };
            text_input::Style {
                background: Background::Color(SURFACE0),
                border: Border::default().rounded(6).width(1.0).color(border_col),
                icon: SUBTEXT0,
                placeholder: OVERLAY0,
                value: TEXT,
                selection: LAVENDER,
            }
        });

    let hex_row = row![text("#").size(14).color(SUBTEXT0), hex_field]
        .spacing(2)
        .align_y(iced::Alignment::Center);

    // Top row: preview + hex + spacer + optional remove
    let top_row: Element<'_, Message> = if can_remove {
        row![
            preview,
            hex_row,
            Space::new().width(Length::Fill),
            button(text("Remove").size(11).center().color(RED))
                .padding([4, 12])
                .on_press(Message::RemoveColor(idx))
                .style(|_theme: &Theme, status| {
                    let bg = if status == button::Status::Hovered {
                        Color { a: 0.12, ..RED }
                    } else {
                        Color::TRANSPARENT
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        border: Border::default()
                            .rounded(6)
                            .width(1.0)
                            .color(Color { a: 0.25, ..RED }),
                        ..button::Style::default()
                    }
                }),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .into()
    } else {
        row![preview, hex_row]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .into()
    };

    // RGB channel sliders
    let sliders = column![
        channel_slider("R", r, RED, Message::SetColorR),
        channel_slider("G", g, GREEN, Message::SetColorG),
        channel_slider("B", b, LAVENDER, Message::SetColorB),
    ]
    .spacing(4);

    let content = column![top_row, sliders].spacing(12);

    container(content)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(MANTLE)),
            border: Border::default()
                .rounded(10)
                .width(1.0)
                .color(SURFACE0),
            ..container::Style::default()
        })
        .padding([12, 14])
        .width(Length::Fill)
        .into()
}

/// Single RGB channel slider: label, slider bar, numeric value.
fn channel_slider<'a>(
    label: &'a str,
    value: u8,
    color: Color,
    on_change: fn(u8) -> Message,
) -> Element<'a, Message> {
    row![
        text(label).size(12).color(color).width(16),
        slider(0..=255, value, on_change).width(Length::Fill),
        text(format!("{value:>3}"))
            .size(11)
            .color(SUBTEXT0)
            .width(28),
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
