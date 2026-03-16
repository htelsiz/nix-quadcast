//! Sliglight — iced GUI for HyperX QuadCast RGB control.

mod engine;
mod mic_preview;

use iced::widget::{column, container, text};
use iced::{Element, Length, Subscription, Task, Theme};

use sliglight_core::animations::{Mode, Zone};

fn main() -> iced::Result {
    env_logger::init();
    iced::application(boot, update, view)
        .title("Sliglight")
        .theme(Theme::CatppuccinMocha)
        .subscription(subscription)
        .window_size((680.0, 750.0))
        .run()
}

struct App {
    zone: Zone,
    mode: Mode,
    brightness: u8,
    speed: u8,
    colors: Vec<(u8, u8, u8)>,
    status: Status,
    engine: Option<engine::Handle>,
}

enum Status {
    Idle,
    Connected,
    Error(String),
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Variants used in Task 8 (full GUI layout)
enum Message {
    SetZone(Zone),
    SetMode(Mode),
    SetBrightness(u8),
    SetSpeed(u8),
    AddColor,
    RemoveColor(usize),
    SetColor(usize, (u8, u8, u8)),
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
            status: Status::Idle,
            engine: None,
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
            }
        }
        Message::SetColor(i, c) => {
            if i < app.colors.len() {
                app.colors[i] = c;
            }
        }
        Message::Apply => {
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
            app.engine = None;
            app.status = Status::Idle;
        }
        Message::EngineEvent(e) => match e {
            engine::Event::Connected => app.status = Status::Connected,
            engine::Event::Error(msg) => app.status = Status::Error(msg),
            engine::Event::FrameSent { .. } => {}
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
    // TODO: build full UI in Task 8
    let status_text = match &app.status {
        Status::Idle => "Ready".to_string(),
        Status::Connected => "Connected to QuadCast 2S".to_string(),
        Status::Error(e) => format!("Error: {e}"),
    };

    let content = column![text("Sliglight").size(24), text(status_text),]
        .spacing(20)
        .padding(20);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
