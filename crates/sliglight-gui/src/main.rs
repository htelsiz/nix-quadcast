//! Sliglight — iced GUI for HyperX QuadCast RGB control.

mod engine;
mod mic_preview;

use iced::widget::{button, canvas::Canvas, column, container, row, slider, text, Space};
use iced::{Background, Border, Color, Element, Length, Subscription, Task, Theme};

use mic_preview::MicPreview;

use sliglight_core::animations::{Mode, Zone};

fn main() -> iced::Result {
    env_logger::init();
    iced::application(boot, update, view)
        .title("Sliglight")
        .theme(Theme::CatppuccinMocha)
        .subscription(subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(900.0, 750.0),
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

/// Default LED preview color (dark gray, matches unlit mic).
const DEFAULT_PREVIEW: Color = Color::from_rgb(
    0x31 as f32 / 255.0,
    0x32 as f32 / 255.0,
    0x44 as f32 / 255.0,
);

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
                // Close editor if the removed color was being edited, or adjust index
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

fn view(app: &App) -> Element<'_, Message> {
    let mic = container(
        Canvas::new(MicPreview {
            upper_color: app.upper_preview_color,
            lower_color: app.lower_preview_color,
        })
        .width(200)
        .height(400),
    )
    .padding(24);

    let controls = column![
        view_title(),
        view_zone_selector(app),
        view_mode_grid(app),
        view_sliders(app),
        view_color_palette(app),
        view_actions(),
        Space::new().height(Length::Fill),
        view_status(app),
    ]
    .spacing(16)
    .padding(24)
    .width(Length::Fill);

    container(row![mic, controls])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// View helpers
// ---------------------------------------------------------------------------

fn view_title() -> Element<'static, Message> {
    text("Sliglight").size(28).into()
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
            button(text(*label).center())
                .width(Length::Fill)
                .on_press(Message::SetZone(z))
                .style(move |theme, status| zone_btn_style(theme, status, selected))
                .into()
        })
        .collect();

    column![
        text("Zone").size(16),
        row(buttons).spacing(8).width(Length::Fill),
    ]
    .spacing(6)
    .into()
}

fn view_mode_grid(app: &App) -> Element<'_, Message> {
    let modes = Mode::ALL;
    // 2 rows x 3 columns
    let top_row: Vec<Element<'_, Message>> = modes[..3]
        .iter()
        .map(|m| mode_button(app, *m))
        .collect();
    let bottom_row: Vec<Element<'_, Message>> = modes[3..]
        .iter()
        .map(|m| mode_button(app, *m))
        .collect();

    column![
        text("Mode").size(16),
        row(top_row).spacing(8).width(Length::Fill),
        row(bottom_row).spacing(8).width(Length::Fill),
    ]
    .spacing(6)
    .into()
}

fn mode_button<'a>(app: &App, mode: Mode) -> Element<'a, Message> {
    let selected = app.mode == mode;
    let label = format!("{} {}", mode.icon(), mode.name());
    button(text(label).center())
        .width(Length::Fill)
        .on_press(Message::SetMode(mode))
        .style(move |theme, status| zone_btn_style(theme, status, selected))
        .into()
}

fn view_sliders(app: &App) -> Element<'_, Message> {
    let brightness_label = format!("Brightness: {}", app.brightness);
    let speed_label = format!("Speed: {}", app.speed);

    column![
        text("Settings").size(16),
        column![
            text(brightness_label).size(14),
            slider(0..=100, app.brightness, Message::SetBrightness),
        ]
        .spacing(4),
        column![
            text(speed_label).size(14),
            slider(0..=100, app.speed, Message::SetSpeed),
        ]
        .spacing(4),
    ]
    .spacing(8)
    .into()
}

fn view_color_palette(app: &App) -> Element<'_, Message> {
    let mut items: Vec<Element<'_, Message>> = Vec::new();

    for (i, &(r, g, b)) in app.colors.iter().enumerate() {
        let is_editing = app.editing_color == Some(i);
        let swatch = button(Space::new().width(24).height(24))
            .style(move |_theme, _status| swatch_style(r, g, b, is_editing))
            .on_press(Message::ToggleColorEditor(i));

        if i == 0 {
            // First color cannot be removed
            items.push(swatch.into());
        } else {
            let remove = button(text("\u{00d7}").size(12).center())
                .on_press(Message::RemoveColor(i))
                .padding([2, 6]);
            items.push(
                column![swatch, remove]
                    .spacing(2)
                    .align_x(iced::Alignment::Center)
                    .into(),
            );
        }
    }

    if app.colors.len() < 11 {
        let add_btn = button(text("+").size(18).center())
            .width(32)
            .height(32)
            .on_press(Message::AddColor);
        items.push(add_btn.into());
    }

    let mut palette = column![
        text("Colors").size(16),
        row(items).spacing(8).align_y(iced::Alignment::End),
    ]
    .spacing(6);

    // Inline color editor when a swatch is selected
    if let Some(idx) = app.editing_color {
        if idx < app.colors.len() {
            palette = palette.push(view_color_editor(app.colors[idx]));
        }
    }

    palette.into()
}

fn view_color_editor((r, g, b): (u8, u8, u8)) -> Element<'static, Message> {
    let preview = container(Space::new().width(48).height(48))
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(r, g, b))),
            border: Border::default().rounded(6).width(2).color(Color::from_rgb(0.4, 0.4, 0.4)),
            ..container::Style::default()
        });

    let hex_label = text(format!("#{r:02X}{g:02X}{b:02X}")).size(13);

    let r_slider = column![
        text(format!("R: {r}")).size(13).color(Color::from_rgb(0.9, 0.4, 0.4)),
        slider(0..=255, r, Message::SetColorR),
    ]
    .spacing(2);

    let g_slider = column![
        text(format!("G: {g}")).size(13).color(Color::from_rgb(0.4, 0.9, 0.4)),
        slider(0..=255, g, Message::SetColorG),
    ]
    .spacing(2);

    let b_slider = column![
        text(format!("B: {b}")).size(13).color(Color::from_rgb(0.4, 0.5, 0.9)),
        slider(0..=255, b, Message::SetColorB),
    ]
    .spacing(2);

    let sliders = column![r_slider, g_slider, b_slider].spacing(6).width(Length::Fill);

    let preview_col = column![preview, hex_label]
        .spacing(4)
        .align_x(iced::Alignment::Center);

    container(row![preview_col, sliders].spacing(16).align_y(iced::Alignment::Center))
        .padding([8, 0])
        .into()
}

fn view_actions() -> Element<'static, Message> {
    row![
        button(text("Apply").center())
            .width(Length::Fill)
            .on_press(Message::Apply)
            .style(|theme, status| apply_btn_style(theme, status)),
        button(text("Reset").center())
            .width(Length::Fill)
            .on_press(Message::Reset),
    ]
    .spacing(12)
    .into()
}

fn view_status(app: &App) -> Element<'_, Message> {
    let (label, color) = match &app.status {
        Status::Idle => ("Ready", Color::from_rgb(0.6, 0.6, 0.6)),
        Status::Connected => (
            "Connected to QuadCast 2S",
            Color::from_rgb(0.4, 0.9, 0.4),
        ),
        Status::Error(_) => ("Error", Color::from_rgb(0.9, 0.3, 0.3)),
    };
    let status_text = match &app.status {
        Status::Error(e) => format!("Error: {e}"),
        _ => label.to_string(),
    };
    container(text(status_text).size(13).color(color))
        .width(Length::Fill)
        .padding([8, 0])
        .into()
}

// ---------------------------------------------------------------------------
// Button styles
// ---------------------------------------------------------------------------

fn zone_btn_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let palette = theme.extended_palette();
    let mut style = button::Style {
        border: Border::default().rounded(6),
        ..button::Style::default()
    };

    if selected {
        style.background = Some(Background::Color(palette.primary.base.color));
        style.text_color = palette.primary.base.text;
    } else {
        style.background = Some(Background::Color(palette.background.weak.color));
        style.text_color = palette.background.weak.text;
        if status == button::Status::Hovered {
            style.background = Some(Background::Color(palette.background.strong.color));
        }
    }
    style
}

fn swatch_style(r: u8, g: u8, b: u8, selected: bool) -> button::Style {
    let border_color = if selected {
        Color::WHITE
    } else {
        Color::from_rgb(0.3, 0.3, 0.3)
    };
    let border_width = if selected { 3.0 } else { 2.0 };
    button::Style {
        background: Some(Background::Color(Color::from_rgb8(r, g, b))),
        border: Border::default()
            .rounded(4)
            .width(border_width)
            .color(border_color),
        ..button::Style::default()
    }
}

fn apply_btn_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let mut style = button::Style {
        border: Border::default().rounded(6),
        background: Some(Background::Color(palette.success.base.color)),
        text_color: palette.success.base.text,
        ..button::Style::default()
    };
    if status == button::Status::Hovered {
        style.background = Some(Background::Color(palette.success.strong.color));
        style.text_color = palette.success.strong.text;
    }
    style
}
