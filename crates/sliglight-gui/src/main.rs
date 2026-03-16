//! Sliglight — iced GUI for HyperX QuadCast RGB control.

mod audio;
mod dbus_service;
mod engine;
mod mic_preview;
mod screen_lock;
mod tray;

use iced::widget::{
    button, canvas::Canvas, column, container, pick_list, row, scrollable, slider, text,
    text_input, tooltip, Space,
};
use iced::{Background, Border, Color, Element, Font, Length, Padding, Shadow, Subscription, Task, Theme};

use mic_preview::MicPreview;

use sliglight_core::animations::{Mode, Zone};
use sliglight_core::config::{AppConfig, Profile};

use tokio::sync::watch;

// ---------------------------------------------------------------------------
// Palette — Catppuccin Mocha accents used for custom styling
// ---------------------------------------------------------------------------

const SURFACE0: Color =
    Color::from_rgb(0x31 as f32 / 255.0, 0x32 as f32 / 255.0, 0x47 as f32 / 255.0);
const SURFACE1: Color =
    Color::from_rgb(0x45 as f32 / 255.0, 0x47 as f32 / 255.0, 0x5A as f32 / 255.0);
const SURFACE2: Color =
    Color::from_rgb(0x58 as f32 / 255.0, 0x5B as f32 / 255.0, 0x70 as f32 / 255.0);
const OVERLAY0: Color =
    Color::from_rgb(0x6C as f32 / 255.0, 0x70 as f32 / 255.0, 0x86 as f32 / 255.0);
const SUBTEXT0: Color =
    Color::from_rgb(0xA6 as f32 / 255.0, 0xAD as f32 / 255.0, 0xC8 as f32 / 255.0);
const TEXT: Color =
    Color::from_rgb(0xCD as f32 / 255.0, 0xD6 as f32 / 255.0, 0xF4 as f32 / 255.0);
const LAVENDER: Color =
    Color::from_rgb(0xB4 as f32 / 255.0, 0xBE as f32 / 255.0, 0xFE as f32 / 255.0);
const GREEN: Color =
    Color::from_rgb(0xA6 as f32 / 255.0, 0xE3 as f32 / 255.0, 0xA1 as f32 / 255.0);
const RED: Color =
    Color::from_rgb(0xF3 as f32 / 255.0, 0x8B as f32 / 255.0, 0xA8 as f32 / 255.0);
const YELLOW: Color =
    Color::from_rgb(0xF9 as f32 / 255.0, 0xE2 as f32 / 255.0, 0xAF as f32 / 255.0);
const BASE: Color =
    Color::from_rgb(0x1E as f32 / 255.0, 0x1E as f32 / 255.0, 0x2E as f32 / 255.0);
const MANTLE: Color =
    Color::from_rgb(0x18 as f32 / 255.0, 0x18 as f32 / 255.0, 0x25 as f32 / 255.0);

/// Default LED preview color (matches unlit mic body).
const DEFAULT_PREVIEW: Color = SURFACE0;

/// Iosevka monospace font for code blocks.
const IOSEVKA: Font = Font {
    family: iced::font::Family::Name("Iosevka"),
    ..Font::MONOSPACE
};

// ---------------------------------------------------------------------------
// Reusable style helpers
// ---------------------------------------------------------------------------

fn card<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(MANTLE)),
            border: Border::default()
                .rounded(12)
                .width(1.0)
                .color(SURFACE0),
            ..container::Style::default()
        })
        .padding(16)
        .width(Length::Fill)
        .into()
}

fn scrollable_style() -> scrollable::Style {
    let rail = scrollable::Rail {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(SURFACE1),
            border: Border::default().rounded(4),
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(SURFACE0),
            border: Border::default().rounded(4),
            shadow: Shadow::default(),
            icon: SUBTEXT0,
        },
    }
}

fn accent_slider_style(_theme: &Theme, status: slider::Status) -> slider::Style {
    let handle_color = match status {
        slider::Status::Hovered => LAVENDER,
        slider::Status::Dragged => Color { a: 0.9, ..LAVENDER },
        _ => SUBTEXT0,
    };
    slider::Style {
        rail: slider::Rail {
            backgrounds: (Background::Color(LAVENDER), Background::Color(SURFACE0)),
            width: 4.0,
            border: Border::default().rounded(2),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 7.0 },
            background: Background::Color(handle_color),
            border_width: 2.0,
            border_color: SURFACE0,
        },
    }
}

fn main() -> iced::Result {
    env_logger::init();
    iced::application(boot, update, view)
        .title("Sliglight")
        .theme(Theme::CatppuccinMocha)
        .subscription(subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(1100.0, 740.0),
            icon: load_icon(),
            ..Default::default()
        })
        .run()
}

fn load_icon() -> Option<iced::window::icon::Icon> {
    let svg_data = include_bytes!("../../../resources/sliglight.svg");
    let tree =
        resvg::usvg::Tree::from_data(svg_data, &resvg::usvg::Options::default()).ok()?;
    let size = 64u32;
    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    let svg_size = tree.size();
    let scale = size as f32 / svg_size.width().max(svg_size.height());
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    iced::window::icon::from_rgba(pixmap.data().to_vec(), size, size).ok()
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct App {
    config: AppConfig,
    // Active editing state (from active profile)
    zone: Zone,
    mode: Mode,
    brightness: u8,
    speed: u8,
    colors: Vec<(u8, u8, u8)>,
    // UI state
    editing_color: Option<usize>,
    hex_input: String,
    profile_name_input: String,
    upper_preview_color: Color,
    lower_preview_color: Color,
    // Engine
    engine_tx: Option<watch::Sender<engine::EngineConfig>>,
    connection: ConnectionState,
    // System integration
    dbus_state_tx: Option<watch::Sender<dbus_service::DbusState>>,
    tray_state_tx: Option<watch::Sender<tray::TrayState>>,
    screen_locked: bool,
    pre_lock_config: Option<engine::EngineConfig>,
    // Audio
    is_muted: bool,
    pre_mute_config: Option<engine::EngineConfig>,
    // Import/export
    export_text: String,
    import_text: String,
    // Diagnostics
    show_diagnostics: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum ConnectionState {
    Starting,
    Scanning,
    Connected,
    Disconnected,
    Error(String),
}

#[derive(Debug, Clone)]
enum Message {
    // Controls
    SetZone(Zone),
    SetMode(Mode),
    SetBrightness(u8),
    SetSpeed(u8),
    // Colors
    AddColor,
    RemoveColor(usize),
    ToggleColorEditor(usize),
    HexInputChanged(String),
    SetColorR(u8),
    SetColorG(u8),
    SetColorB(u8),
    // Profiles
    SelectProfile(String),
    SaveProfileAs,
    DeleteProfile,
    ProfileNameInput(String),
    ExportProfile,
    ImportProfile,
    CopyProfile,
    ImportTomlInput(String),
    // Diagnostics
    ToggleDiagnostics,
    CopyConfig,
    // Engine
    Engine(engine::Event),
    // Audio
    Audio(audio::Event),
    // System integration
    Dbus(dbus_service::Event),
    ScreenLock(screen_lock::Event),
    Tray(tray::Event),
}

fn boot() -> (App, Task<Message>) {
    let config = AppConfig::load();
    let profile = config.active_profile().cloned().unwrap_or(Profile {
        mode: Mode::Solid,
        zone: Zone::Both,
        brightness: 80,
        speed: 81,
        colors: vec![(255, 0, 0)],
    });

    (
        App {
            zone: profile.zone,
            mode: profile.mode,
            brightness: profile.brightness,
            speed: profile.speed,
            colors: if profile.colors.is_empty() {
                vec![(255, 0, 0)]
            } else {
                profile.colors.clone()
            },
            editing_color: None,
            hex_input: String::new(),
            profile_name_input: String::new(),
            upper_preview_color: DEFAULT_PREVIEW,
            lower_preview_color: DEFAULT_PREVIEW,
            engine_tx: None,
            connection: ConnectionState::Starting,
            dbus_state_tx: None,
            tray_state_tx: None,
            screen_locked: false,
            pre_lock_config: None,
            is_muted: false,
            pre_mute_config: None,
            export_text: String::new(),
            import_text: String::new(),
            show_diagnostics: false,
            config,
        },
        Task::none(),
    )
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::SetZone(z) => {
            app.zone = z;
            push_config(app);
        }
        Message::SetMode(m) => {
            app.mode = m;
            push_config(app);
        }
        Message::SetBrightness(b) => {
            app.brightness = b;
            push_config(app);
        }
        Message::SetSpeed(s) => {
            app.speed = s;
            push_config(app);
        }
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
                push_config(app);
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
                push_config(app);
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
                            push_config(app);
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
                    push_config(app);
                }
            }
        }
        Message::SetColorG(v) => {
            if let Some(idx) = app.editing_color {
                if idx < app.colors.len() {
                    app.colors[idx].1 = v;
                    let (r, g, b) = app.colors[idx];
                    app.hex_input = format!("{r:02X}{g:02X}{b:02X}");
                    push_config(app);
                }
            }
        }
        Message::SetColorB(v) => {
            if let Some(idx) = app.editing_color {
                if idx < app.colors.len() {
                    app.colors[idx].2 = v;
                    let (r, g, b) = app.colors[idx];
                    app.hex_input = format!("{r:02X}{g:02X}{b:02X}");
                    push_config(app);
                }
            }
        }
        Message::SelectProfile(name) => {
            app.config.active_profile = name;
            load_active_profile(app);
            push_config(app);
        }
        Message::SaveProfileAs => {
            let name = app.profile_name_input.trim().to_string();
            if !name.is_empty() {
                let profile = current_profile(app);
                app.config.profiles.insert(name.clone(), profile);
                app.config.active_profile = name;
                app.config.save();
                app.profile_name_input.clear();
            }
        }
        Message::DeleteProfile => {
            let name = app.config.active_profile.clone();
            app.config.profiles.remove(&name);
            // Switch to first remaining profile.
            if let Some(next) = app.config.profiles.keys().next().cloned() {
                app.config.active_profile = next;
            }
            load_active_profile(app);
            push_config(app);
        }
        Message::ProfileNameInput(s) => {
            app.profile_name_input = s;
        }
        Message::ExportProfile => {
            let profile = current_profile(app);
            app.export_text = profile.to_json().unwrap_or_default();
        }
        Message::ImportProfile => {
            if let Ok(profile) = Profile::from_json(&app.import_text) {
                let name = format!("Imported {}", app.config.profiles.len() + 1);
                app.config.profiles.insert(name.clone(), profile.clone());
                app.config.active_profile = name;
                apply_profile(app, &profile);
                push_config(app);
                app.import_text.clear();
            }
        }
        Message::CopyProfile => {
            let name = format!("{} (copy)", app.config.active_profile);
            let profile = current_profile(app);
            app.config.profiles.insert(name.clone(), profile);
            app.config.active_profile = name;
            app.config.save();
        }
        Message::ImportTomlInput(s) => {
            app.import_text = s;
        }
        Message::ToggleDiagnostics => {
            app.show_diagnostics = !app.show_diagnostics;
        }
        Message::CopyConfig => {
            return iced::clipboard::write(app.config.to_json());
        }
        Message::Engine(e) => match e {
            engine::Event::Ready(tx) => {
                app.engine_tx = Some(tx);
                push_config(app);
            }
            engine::Event::Connected => {
                app.connection = ConnectionState::Connected;
                push_dbus_state(app);
                push_tray_state(app);
            }
            engine::Event::Disconnected => {
                app.connection = ConnectionState::Disconnected;
                push_dbus_state(app);
                push_tray_state(app);
            }
            engine::Event::Reconnecting => {
                app.connection = ConnectionState::Scanning;
            }
            engine::Event::FrameSent { upper, lower } => {
                app.upper_preview_color = Color::from_rgb8(upper.0, upper.1, upper.2);
                app.lower_preview_color = Color::from_rgb8(lower.0, lower.1, lower.2);
            }
            engine::Event::Error(msg) => {
                app.connection = ConnectionState::Error(msg);
                push_dbus_state(app);
                push_tray_state(app);
            }
        },
        Message::Audio(e) => match e {
            audio::Event::MuteChanged(muted) => {
                app.is_muted = muted;
                if muted && app.config.mute_indicator_enabled {
                    // Save current config and override to solid red.
                    app.pre_mute_config = Some(engine::EngineConfig {
                        mode: app.mode,
                        colors: app.colors.clone(),
                        brightness: app.brightness,
                        speed: app.speed,
                        zone: app.zone,
                    });
                    if let Some(tx) = &app.engine_tx {
                        let _ = tx.send(engine::EngineConfig {
                            mode: Mode::Solid,
                            colors: vec![(255, 0, 0)],
                            brightness: 100,
                            speed: 0,
                            zone: Zone::Both,
                        });
                    }
                } else if !muted {
                    // Restore previous config.
                    if let Some(cfg) = app.pre_mute_config.take() {
                        if let Some(tx) = &app.engine_tx {
                            let _ = tx.send(cfg);
                        }
                    }
                }
                push_tray_state(app);
            }
        },
        Message::Dbus(e) => match e {
            dbus_service::Event::Ready(tx) => {
                app.dbus_state_tx = Some(tx);
                push_dbus_state(app);
            }
            dbus_service::Event::Command(cmd) => match cmd {
                dbus_service::Command::SetProfile(name) => {
                    app.config.active_profile = name;
                    load_active_profile(app);
                    push_config(app);
                }
                dbus_service::Command::SetMode(name) => {
                    if let Some(m) = Mode::from_name(&name) {
                        app.mode = m;
                        push_config(app);
                    }
                }
                dbus_service::Command::SetBrightness(b) => {
                    app.brightness = b.min(100);
                    push_config(app);
                }
                dbus_service::Command::SetColor(hex) => {
                    let hex = hex.trim_start_matches('#');
                    if hex.len() == 6 {
                        if let (Ok(r), Ok(g), Ok(b)) = (
                            u8::from_str_radix(&hex[0..2], 16),
                            u8::from_str_radix(&hex[2..4], 16),
                            u8::from_str_radix(&hex[4..6], 16),
                        ) {
                            app.colors = vec![(r, g, b)];
                            push_config(app);
                        }
                    }
                }
            },
        },
        Message::ScreenLock(e) => match e {
            screen_lock::Event::Locked => {
                if app.config.screen_lock_blackout {
                    app.screen_locked = true;
                    // Save current config and send blackout.
                    app.pre_lock_config = Some(engine::EngineConfig {
                        mode: app.mode,
                        colors: app.colors.clone(),
                        brightness: app.brightness,
                        speed: app.speed,
                        zone: app.zone,
                    });
                    if let Some(tx) = &app.engine_tx {
                        let _ = tx.send(engine::EngineConfig {
                            mode: Mode::Solid,
                            colors: vec![(0, 0, 0)],
                            brightness: 0,
                            speed: 0,
                            zone: Zone::Both,
                        });
                    }
                }
            }
            screen_lock::Event::Unlocked => {
                if app.screen_locked {
                    app.screen_locked = false;
                    if let Some(cfg) = app.pre_lock_config.take() {
                        if let Some(tx) = &app.engine_tx {
                            let _ = tx.send(cfg);
                        }
                    }
                }
            }
        },
        Message::Tray(e) => match e {
            tray::Event::Ready(tx) => {
                app.tray_state_tx = Some(tx);
                push_tray_state(app);
            }
            tray::Event::Command(cmd) => match cmd {
                tray::Command::SelectProfile(name) => {
                    app.config.active_profile = name;
                    load_active_profile(app);
                    push_config(app);
                }
                tray::Command::ToggleOnOff => {
                    if app.brightness > 0 {
                        app.brightness = 0;
                    } else {
                        app.brightness = 80;
                    }
                    push_config(app);
                }
                tray::Command::Quit => {
                    return iced::exit();
                }
            },
        },
    }
    Task::none()
}

/// Build an EngineConfig from current app state and send it to the engine.
fn push_config(app: &mut App) {
    if let Some(tx) = &app.engine_tx {
        let _ = tx.send(engine::EngineConfig {
            mode: app.mode,
            colors: app.colors.clone(),
            brightness: app.brightness,
            speed: app.speed,
            zone: app.zone,
        });
    }
    // Persist the active profile.
    let profile = current_profile(app);
    app.config
        .profiles
        .insert(app.config.active_profile.clone(), profile);
    app.config.save();
    push_dbus_state(app);
    push_tray_state(app);
}

fn current_profile(app: &App) -> Profile {
    Profile {
        mode: app.mode,
        zone: app.zone,
        brightness: app.brightness,
        speed: app.speed,
        colors: app.colors.clone(),
    }
}

fn load_active_profile(app: &mut App) {
    if let Some(profile) = app.config.active_profile().cloned() {
        apply_profile(app, &profile);
    }
}

fn apply_profile(app: &mut App, profile: &Profile) {
    app.mode = profile.mode;
    app.zone = profile.zone;
    app.brightness = profile.brightness;
    app.speed = profile.speed;
    app.colors = if profile.colors.is_empty() {
        vec![(255, 0, 0)]
    } else {
        profile.colors.clone()
    };
    app.editing_color = None;
    app.hex_input.clear();
}

fn push_dbus_state(app: &App) {
    if let Some(tx) = &app.dbus_state_tx {
        let mut names: Vec<String> = app.config.profiles.keys().cloned().collect();
        names.sort();
        let _ = tx.send(dbus_service::DbusState {
            current_profile: app.config.active_profile.clone(),
            is_connected: app.connection == ConnectionState::Connected,
            profile_names: names,
        });
    }
}

fn push_tray_state(app: &App) {
    if let Some(tx) = &app.tray_state_tx {
        let mut names: Vec<String> = app.config.profiles.keys().cloned().collect();
        names.sort();
        let _ = tx.send(tray::TrayState {
            profile_names: names,
            active_profile: app.config.active_profile.clone(),
            is_connected: app.connection == ConnectionState::Connected,
            is_on: app.brightness > 0,
        });
    }
}

fn subscription(_app: &App) -> Subscription<Message> {
    Subscription::batch(vec![
        engine::subscription().map(Message::Engine),
        audio::subscription().map(Message::Audio),
        dbus_service::subscription().map(Message::Dbus),
        screen_lock::subscription().map(Message::ScreenLock),
        tray::subscription().map(Message::Tray),
    ])
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

fn view(app: &App) -> Element<'_, Message> {
    let mic = container(
        column![
            Canvas::new(MicPreview {
                upper_color: app.upper_preview_color,
                lower_color: app.lower_preview_color,
            })
            .width(240)
            .height(480),
            text(format!(
                "{} \u{00B7} {}",
                app.mode.name(),
                match app.zone {
                    Zone::Both => "Both",
                    Zone::Upper => "Upper",
                    Zone::Lower => "Lower",
                }
            ))
            .size(11)
            .color(OVERLAY0)
            .center(),
        ]
        .spacing(8)
        .align_x(iced::Alignment::Center),
    )
    .padding([20, 16])
    .center_y(Length::Fill);

    let controls = scrollable(
        column![
            view_header(),
            Space::new().height(4),
            card(view_profile_selector(app)),
            card(column![view_zone_selector(app), view_mode_grid(app)].spacing(12)),
            card(view_sliders(app)),
            card(view_color_palette(app)),
            Space::new().height(Length::Fill),
            view_diagnostics(app),
            view_status(app),
        ]
        .spacing(10)
        .padding([20, 24])
        .width(Length::Fill),
    )
    .style(|_theme: &Theme, _status| scrollable_style());

    // Vertical separator between mic and controls
    let separator = container(
        container(Space::new().width(1).height(Length::Fill)).style(
            |_theme: &Theme| container::Style {
                background: Some(Background::Color(SURFACE0)),
                ..container::Style::default()
            },
        ),
    )
    .padding([40, 0])
    .height(Length::Fill);

    let sep2 = container(
        container(Space::new().width(1).height(Length::Fill)).style(
            |_theme: &Theme| container::Style {
                background: Some(Background::Color(SURFACE0)),
                ..container::Style::default()
            },
        ),
    )
    .padding([40, 0])
    .height(Length::Fill);

    let main_row = row![mic, separator, controls, sep2, view_config_panel(app)];

    container(main_row)
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
    text(label.to_uppercase()).size(11).color(OVERLAY0).into()
}

fn view_header() -> Element<'static, Message> {
    column![
        text("Sliglight").size(26).color(TEXT),
        text("QuadCast 2S RGB Control").size(13).color(SURFACE2),
    ]
    .spacing(4)
    .into()
}

fn view_profile_selector(app: &App) -> Element<'_, Message> {
    let mut profile_names: Vec<String> = app.config.profiles.keys().cloned().collect();
    profile_names.sort();

    let picker = pick_list(profile_names, Some(app.config.active_profile.clone()), |name| {
        Message::SelectProfile(name)
    })
    .text_size(13)
    .width(Length::Fill)
    .style(|_theme: &Theme, _status| pick_list::Style {
        text_color: TEXT,
        placeholder_color: OVERLAY0,
        handle_color: SUBTEXT0,
        background: Background::Color(SURFACE0),
        border: Border::default().rounded(8).width(1.0).color(SURFACE1),
    });

    let save_input = text_input("New profile name...", &app.profile_name_input)
        .on_input(Message::ProfileNameInput)
        .on_submit(Message::SaveProfileAs)
        .size(12)
        .width(140)
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

    let save_btn = button(text("Save").size(11).center().color(GREEN))
        .padding([4, 10])
        .on_press(Message::SaveProfileAs)
        .style(|_theme: &Theme, status| {
            let bg = if status == button::Status::Hovered {
                Color { a: 0.15, ..GREEN }
            } else {
                Color::TRANSPARENT
            };
            button::Style {
                background: Some(Background::Color(bg)),
                border: Border::default()
                    .rounded(6)
                    .width(1.0)
                    .color(Color { a: 0.3, ..GREEN }),
                ..button::Style::default()
            }
        });

    let delete_btn = button(text("Delete").size(11).center().color(RED))
        .padding([4, 10])
        .on_press_maybe(if app.config.profiles.len() > 1 {
            Some(Message::DeleteProfile)
        } else {
            None
        })
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
        });

    let copy_btn = button(text("Copy").size(11).center().color(LAVENDER))
        .padding([4, 10])
        .on_press(Message::CopyProfile)
        .style(|_theme: &Theme, status| {
            let bg = if status == button::Status::Hovered {
                Color { a: 0.12, ..LAVENDER }
            } else {
                Color::TRANSPARENT
            };
            button::Style {
                background: Some(Background::Color(bg)),
                border: Border::default()
                    .rounded(6)
                    .width(1.0)
                    .color(Color { a: 0.25, ..LAVENDER }),
                ..button::Style::default()
            }
        });

    let export_btn = button(text("Export").size(11).center().color(SUBTEXT0))
        .padding([4, 10])
        .on_press(Message::ExportProfile)
        .style(|_theme: &Theme, status| {
            let bg = if status == button::Status::Hovered {
                SURFACE1
            } else {
                Color::TRANSPARENT
            };
            button::Style {
                background: Some(Background::Color(bg)),
                border: Border::default()
                    .rounded(6)
                    .width(1.0)
                    .color(SURFACE1),
                ..button::Style::default()
            }
        });

    column![
        section_label("Profile"),
        picker,
        row![save_input, save_btn, copy_btn, export_btn, Space::new().width(Length::Fill), delete_btn]
            .spacing(6)
            .align_y(iced::Alignment::Center),
    ]
    .spacing(6)
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
    let mid = (modes.len() + 1) / 2; // 4 on top, 3 on bottom for 7 modes
    let top_row: Vec<Element<'_, Message>> = modes[..mid]
        .iter()
        .map(|m| mode_button(app, *m))
        .collect();
    let bottom_row: Vec<Element<'_, Message>> = modes[mid..]
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
    let name = mode.name();
    let label = format!("{}{}", &name[..1].to_uppercase(), &name[1..]);
    let btn = button(text(label).size(13).center())
        .width(Length::Fill)
        .height(32)
        .on_press(Message::SetMode(mode))
        .style(move |theme, status| pill_btn_style(theme, status, selected));

    tooltip(btn, mode.description(), tooltip::Position::Bottom)
        .gap(4)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(SURFACE1)),
            border: Border::default().rounded(6).width(1.0).color(SURFACE2),
            text_color: Some(SUBTEXT0),
            ..container::Style::default()
        })
        .into()
}

fn view_sliders(app: &App) -> Element<'_, Message> {
    column![
        section_label("Settings"),
        row![
            text("Brightness")
                .size(12)
                .color(SUBTEXT0)
                .width(Length::Fill),
            text(format!("{}", app.brightness)).size(12).color(TEXT),
        ],
        slider(0..=100, app.brightness, Message::SetBrightness).style(accent_slider_style),
        row![
            text("Speed").size(12).color(SUBTEXT0).width(Length::Fill),
            text(format!("{}", app.speed)).size(12).color(TEXT),
        ],
        slider(0..=100, app.speed, Message::SetSpeed).style(accent_slider_style),
    ]
    .spacing(4)
    .into()
}

fn view_color_palette(app: &App) -> Element<'_, Message> {
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

    if let Some(idx) = app.editing_color {
        if idx < app.colors.len() {
            content = content.push(view_color_editor(app, idx));
            // Import field (shown below editor when editing)
            content = content.push(view_import(app));
        }
    }

    // Export text (shown inside color card when present)
    if !app.export_text.is_empty() {
        content = content.push(view_export(app));
    }

    content.into()
}

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

fn view_color_editor(app: &App, idx: usize) -> Element<'_, Message> {
    let (r, g, b) = app.colors[idx];
    let can_remove = app.colors.len() > 1;

    let preview = container(Space::new().width(40).height(40)).style(
        move |_theme: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(r, g, b))),
            border: Border::default().rounded(8),
            ..container::Style::default()
        },
    );

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

fn view_export(app: &App) -> Element<'_, Message> {
    column![
        section_label("Exported Profile (JSON)"),
        text(&app.export_text).size(11).color(SUBTEXT0),
    ]
    .spacing(4)
    .into()
}

fn view_import(app: &App) -> Element<'_, Message> {
    let import_field = text_input("Paste JSON profile here...", &app.import_text)
        .on_input(Message::ImportTomlInput)
        .size(11)
        .width(Length::Fill)
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

    let import_btn = button(text("Import").size(11).center().color(GREEN))
        .padding([4, 10])
        .on_press(Message::ImportProfile)
        .style(|_theme: &Theme, status| {
            let bg = if status == button::Status::Hovered {
                Color { a: 0.15, ..GREEN }
            } else {
                Color::TRANSPARENT
            };
            button::Style {
                background: Some(Background::Color(bg)),
                border: Border::default()
                    .rounded(6)
                    .width(1.0)
                    .color(Color { a: 0.3, ..GREEN }),
                ..button::Style::default()
            }
        });

    row![import_field, import_btn]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into()
}

fn view_diagnostics(app: &App) -> Element<'_, Message> {
    let toggle_btn = button(
        text(if app.show_diagnostics {
            "Hide Diagnostics"
        } else {
            "Diagnostics"
        })
        .size(11)
        .center()
        .color(SUBTEXT0),
    )
    .padding([4, 10])
    .on_press(Message::ToggleDiagnostics)
    .style(|_theme: &Theme, status| {
        let bg = if status == button::Status::Hovered {
            SURFACE1
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(Background::Color(bg)),
            border: Border::default()
                .rounded(6)
                .width(1.0)
                .color(SURFACE1),
            ..button::Style::default()
        }
    });

    if !app.show_diagnostics {
        return toggle_btn.into();
    }

    let connected_str = match &app.connection {
        ConnectionState::Connected => "Yes",
        ConnectionState::Starting => "Starting",
        ConnectionState::Scanning => "Scanning",
        ConnectionState::Disconnected => "No",
        ConnectionState::Error(_) => "Error",
    };

    let info = column![
        row![
            text("USB VID:PID").size(11).color(OVERLAY0),
            text("03f0:0f8b (HP QuadCast 2S)")
                .size(11)
                .color(SUBTEXT0),
        ]
        .spacing(8),
        row![
            text("Connected").size(11).color(OVERLAY0),
            text(connected_str).size(11).color(SUBTEXT0),
        ]
        .spacing(8),
        row![
            text("Active profile").size(11).color(OVERLAY0),
            text(&app.config.active_profile)
                .size(11)
                .color(SUBTEXT0),
        ]
        .spacing(8),
        row![
            text("Mode").size(11).color(OVERLAY0),
            text(app.mode.name()).size(11).color(SUBTEXT0),
        ]
        .spacing(8),
        row![
            text("Muted").size(11).color(OVERLAY0),
            text(if app.is_muted { "Yes" } else { "No" })
                .size(11)
                .color(if app.is_muted { RED } else { SUBTEXT0 }),
        ]
        .spacing(8),
        row![
            text("Mic level").size(11).color(OVERLAY0),
            text(format!("{:.0}%", engine::get_peak_level() * 100.0))
                .size(11)
                .color(SUBTEXT0),
        ]
        .spacing(8),
        row![
            text("Music level").size(11).color(OVERLAY0),
            text(format!("{:.0}%", engine::get_music_peak_level() * 100.0))
                .size(11)
                .color(SUBTEXT0),
        ]
        .spacing(8),
        row![
            text("udev rule").size(11).color(OVERLAY0),
            text("MODE=0666, TAG+=uaccess").size(11).color(SUBTEXT0),
        ]
        .spacing(8),
    ]
    .spacing(3);

    let panel = container(info)
        .style(|_theme: &Theme| container::Style {
            background: Some(Background::Color(MANTLE)),
            border: Border::default()
                .rounded(8)
                .width(1.0)
                .color(SURFACE0),
            ..container::Style::default()
        })
        .padding([8, 10])
        .width(Length::Fill);

    column![toggle_btn, panel]
        .spacing(4)
        .into()
}

fn view_status(app: &App) -> Element<'_, Message> {
    let (label, dot_color) = match &app.connection {
        ConnectionState::Starting => ("Starting...", SURFACE2),
        ConnectionState::Scanning => ("Scanning for device...", YELLOW),
        ConnectionState::Connected => ("Connected", GREEN),
        ConnectionState::Disconnected => ("Disconnected — reconnecting...", YELLOW),
        ConnectionState::Error(_) => ("Error", RED),
    };
    let status_text = match &app.connection {
        ConnectionState::Error(e) => e.clone(),
        _ => label.to_string(),
    };

    let dot = container(Space::new().width(6).height(6)).style(
        move |_theme: &Theme| container::Style {
            background: Some(Background::Color(dot_color)),
            border: Border::default().rounded(3),
            ..container::Style::default()
        },
    );

    let mute_indicator = if app.is_muted {
        text("MIC MUTED").size(11).color(RED)
    } else {
        text("").size(11)
    };

    container(
        row![
            dot,
            text(status_text).size(12).color(SUBTEXT0),
            Space::new().width(Length::Fill),
            mute_indicator,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .into()
}

// ---------------------------------------------------------------------------
// Button styles
// ---------------------------------------------------------------------------

/// Catppuccin Mocha syntax colors for JSON highlighting.
const SYN_KEY: Color =
    Color::from_rgb(0x89 as f32 / 255.0, 0xB4 as f32 / 255.0, 0xFA as f32 / 255.0); // Blue
const SYN_STRING: Color =
    Color::from_rgb(0xA6 as f32 / 255.0, 0xE3 as f32 / 255.0, 0xA1 as f32 / 255.0); // Green
const SYN_NUMBER: Color =
    Color::from_rgb(0xFA as f32 / 255.0, 0xB3 as f32 / 255.0, 0x87 as f32 / 255.0); // Peach
const SYN_BOOL: Color =
    Color::from_rgb(0xCB as f32 / 255.0, 0xA6 as f32 / 255.0, 0xF7 as f32 / 255.0); // Mauve
const SYN_PUNCT: Color = OVERLAY0;

/// Compact JSON: collapse small arrays (e.g. color tuples) onto single lines.
fn compact_json(json: &str) -> String {
    let mut result = String::with_capacity(json.len());
    let mut lines = json.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        // Detect opening bracket of a small array (next lines are just numbers)
        if trimmed.ends_with('[') {
            let indent = &line[..line.len() - trimmed.len()];
            let mut items: Vec<String> = Vec::new();
            let mut found_close = false;
            // Peek ahead to collect array items
            let mut lookahead: Vec<&str> = Vec::new();
            while let Some(&next) = lines.peek() {
                let nt = next.trim();
                if nt == "]" || nt == "]," {
                    found_close = true;
                    lookahead.push(lines.next().unwrap());
                    break;
                } else if nt.starts_with('{') || nt.starts_with('[') || nt.starts_with('"') {
                    // Nested object/array/string — don't collapse
                    break;
                } else {
                    lookahead.push(lines.next().unwrap());
                    items.push(nt.trim_end_matches(',').to_string());
                }
            }
            if found_close && items.len() <= 5 {
                // Collapse: "colors": [235, 188, 186]
                let close_trim = lookahead.last().unwrap().trim();
                let trailing = if close_trim.ends_with(',') { "," } else { "" };
                result.push_str(line);
                result.push_str(&items.join(", "));
                result.push(']');
                result.push_str(trailing);
                result.push('\n');
            } else {
                // Couldn't collapse — emit original lines
                result.push_str(line);
                result.push('\n');
                for l in &lookahead {
                    result.push_str(l);
                    result.push('\n');
                }
            }
            let _ = indent; // suppress warning
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

/// Build syntax-highlighted JSON as a column of rich text rows.
fn highlighted_json<'a>(json: &str) -> Element<'a, Message> {
    let compact = compact_json(json);
    let mut lines: Vec<Element<'a, Message>> = Vec::new();
    for line in compact.lines() {
        let mut spans: Vec<iced::widget::text::Span<'a, Font>> = Vec::new();
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if indent > 0 {
            spans.push(iced::widget::text::Span {
                text: " ".repeat(indent).into(),
                font: Some(IOSEVKA),
                size: Some(iced::Pixels(11.0)),
                ..Default::default()
            });
        }

        let mut chars = trimmed.chars().peekable();
        let mut buf = String::new();
        while let Some(&ch) = chars.peek() {
            match ch {
                '"' => {
                    buf.clear();
                    buf.push(chars.next().unwrap());
                    let mut escaped = false;
                    for c in chars.by_ref() {
                        buf.push(c);
                        if escaped {
                            escaped = false;
                        } else if c == '\\' {
                            escaped = true;
                        } else if c == '"' {
                            break;
                        }
                    }
                    // Look ahead to see if a colon follows (= this is a key)
                    let is_key = chars.clone().take_while(|c| *c == ' ').count()
                        + if chars.clone().skip_while(|c| *c == ' ').next() == Some(':') { 1 } else { 0 }
                        > 0
                        && chars.clone().skip_while(|c| *c == ' ').next() == Some(':');
                    let color = if is_key { SYN_KEY } else { SYN_STRING };
                    spans.push(iced::widget::text::Span {
                        text: buf.clone().into(),
                        color: Some(color),
                        font: Some(IOSEVKA),
                        size: Some(iced::Pixels(11.0)),
                        ..Default::default()
                    });
                }
                '0'..='9' | '-' => {
                    buf.clear();
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() || c == '.' || c == '-' || c == 'e' || c == 'E' {
                            buf.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    spans.push(iced::widget::text::Span {
                        text: buf.clone().into(),
                        color: Some(SYN_NUMBER),
                        font: Some(IOSEVKA),
                        size: Some(iced::Pixels(11.0)),
                        ..Default::default()
                    });
                }
                't' | 'f' | 'n' => {
                    buf.clear();
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_alphabetic() {
                            buf.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    spans.push(iced::widget::text::Span {
                        text: buf.clone().into(),
                        color: Some(SYN_BOOL),
                        font: Some(IOSEVKA),
                        size: Some(iced::Pixels(11.0)),
                        ..Default::default()
                    });
                }
                '{' | '}' | '[' | ']' | ':' | ',' => {
                    spans.push(iced::widget::text::Span {
                        text: chars.next().unwrap().to_string().into(),
                        color: Some(SYN_PUNCT),
                        font: Some(IOSEVKA),
                        size: Some(iced::Pixels(11.0)),
                        ..Default::default()
                    });
                }
                _ => {
                    spans.push(iced::widget::text::Span {
                        text: chars.next().unwrap().to_string().into(),
                        font: Some(IOSEVKA),
                        size: Some(iced::Pixels(11.0)),
                        ..Default::default()
                    });
                }
            }
        }
        if spans.is_empty() {
            spans.push(iced::widget::text::Span {
                text: " ".into(),
                font: Some(IOSEVKA),
                size: Some(iced::Pixels(11.0)),
                ..Default::default()
            });
        }
        lines.push(iced::widget::rich_text(spans).font(IOSEVKA).size(11).into());
    }
    column(lines).spacing(0).into()
}

fn view_config_panel(app: &App) -> Element<'_, Message> {
    let json = app.config.to_json();

    let header = row![
        text("Live Config").size(11).color(OVERLAY0),
        Space::new().width(Length::Fill),
        config_panel_btn("Copy", Message::CopyConfig, GREEN),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);

    let code_block = scrollable(
        // Right padding 14px gives scrollbar its own lane
        container(highlighted_json(&json)).padding(Padding { top: 8.0, right: 14.0, bottom: 8.0, left: 10.0 }),
    )
    .height(Length::Fill)
    .style(|_theme: &Theme, _status| scrollable_style());

    container(
        column![header, code_block].spacing(6),
    )
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(Color::from_rgb(
            0x14 as f32 / 255.0,
            0x14 as f32 / 255.0,
            0x20 as f32 / 255.0,
        ))),
        ..container::Style::default()
    })
    .padding(Padding { top: 12.0, right: 0.0, bottom: 12.0, left: 12.0 })
    .width(380)
    .height(Length::Fill)
    .into()
}

fn config_panel_btn(label: &'static str, msg: Message, color: Color) -> Element<'static, Message> {
    button(text(label).size(11).center().color(color))
        .padding([4, 10])
        .on_press(msg)
        .style(move |_theme: &Theme, status| {
            let bg = if status == button::Status::Hovered {
                SURFACE1
            } else {
                Color::TRANSPARENT
            };
            button::Style {
                background: Some(Background::Color(bg)),
                border: Border::default()
                    .rounded(6)
                    .width(1.0)
                    .color(SURFACE1),
                ..button::Style::default()
            }
        })
        .into()
}

fn pill_btn_style(_theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    if selected {
        button::Style {
            background: Some(Background::Color(LAVENDER)),
            text_color: BASE,
            border: Border::default()
                .rounded(8)
                .width(2.0)
                .color(Color { a: 0.3, ..LAVENDER }),
            ..button::Style::default()
        }
    } else {
        let (bg, border) = if status == button::Status::Hovered {
            (
                SURFACE1,
                Border::default()
                    .rounded(8)
                    .width(1.0)
                    .color(Color { a: 0.2, ..LAVENDER }),
            )
        } else {
            (SURFACE0, Border::default().rounded(8))
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: SUBTEXT0,
            border,
            ..button::Style::default()
        }
    }
}
