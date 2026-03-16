//! DBus service exposing `org.sliglight.Daemon` on the session bus.
//!
//! Methods: SetProfile, SetMode, SetBrightness, SetColor, ListProfiles
//! Properties: CurrentProfile, IsConnected

use iced::futures::SinkExt;
use iced::Subscription;
use tokio::sync::{mpsc, watch};
use zbus::interface;

/// Commands from DBus → App.
#[derive(Debug, Clone)]
pub enum Command {
    SetProfile(String),
    SetMode(String),
    SetBrightness(u8),
    SetColor(String),
}

/// Events from DBus subscription → App.
#[derive(Debug, Clone)]
pub enum Event {
    Ready(watch::Sender<DbusState>),
    Command(Command),
}

/// State pushed from App → DBus for property queries.
#[derive(Debug, Clone)]
pub struct DbusState {
    pub current_profile: String,
    pub is_connected: bool,
    pub profile_names: Vec<String>,
}

impl Default for DbusState {
    fn default() -> Self {
        Self {
            current_profile: String::new(),
            is_connected: false,
            profile_names: Vec::new(),
        }
    }
}

struct SliglightDbus {
    command_tx: mpsc::Sender<Command>,
    state_rx: watch::Receiver<DbusState>,
}

#[interface(name = "org.sliglight.Daemon")]
impl SliglightDbus {
    async fn set_profile(&self, name: &str) {
        let _ = self
            .command_tx
            .send(Command::SetProfile(name.to_string()))
            .await;
    }

    async fn set_mode(&self, mode: &str) {
        let _ = self
            .command_tx
            .send(Command::SetMode(mode.to_string()))
            .await;
    }

    async fn set_brightness(&self, brightness: u8) {
        let _ = self
            .command_tx
            .send(Command::SetBrightness(brightness))
            .await;
    }

    async fn set_color(&self, hex: &str) {
        let _ = self
            .command_tx
            .send(Command::SetColor(hex.to_string()))
            .await;
    }

    async fn list_profiles(&self) -> Vec<String> {
        self.state_rx.borrow().profile_names.clone()
    }

    #[zbus(property)]
    async fn current_profile(&self) -> String {
        self.state_rx.borrow().current_profile.clone()
    }

    #[zbus(property)]
    async fn is_connected(&self) -> bool {
        self.state_rx.borrow().is_connected
    }
}

pub fn subscription() -> Subscription<Event> {
    Subscription::run(dbus_worker)
}

fn dbus_worker() -> impl iced::futures::Stream<Item = Event> {
    iced::stream::channel(32, async move |mut output| {
        let (command_tx, mut command_rx) = mpsc::channel::<Command>(32);
        let (state_tx, state_rx) = watch::channel(DbusState::default());

        // Send the state sender back to the app so it can push updates.
        let _ = output.send(Event::Ready(state_tx)).await;

        // Spawn the DBus server.
        tokio::spawn(async move {
            let service = SliglightDbus {
                command_tx,
                state_rx,
            };

            match zbus::connection::Builder::session()
                .expect("DBus session builder")
                .name("org.sliglight.Daemon")
                .expect("DBus name")
                .serve_at("/org/sliglight/Daemon", service)
                .expect("DBus serve_at")
                .build()
                .await
            {
                Ok(_conn) => {
                    log::info!("DBus service started at org.sliglight.Daemon");
                    // Keep the connection alive forever.
                    std::future::pending::<()>().await;
                }
                Err(e) => {
                    log::warn!("Failed to start DBus service: {e}");
                }
            }
        });

        // Forward commands from DBus methods to the app as events.
        while let Some(cmd) = command_rx.recv().await {
            let _ = output.send(Event::Command(cmd)).await;
        }
    })
}
