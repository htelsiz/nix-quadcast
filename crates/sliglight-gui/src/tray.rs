//! System tray icon via KDE StatusNotifierItem (ksni).
//!
//! Shows a tray icon with a menu for profile switching and app control.

use iced::futures::SinkExt;
use iced::Subscription;
use ksni::menu::StandardItem;
use ksni::{Tray, TrayMethods};
use tokio::sync::{mpsc, watch};

/// Commands from tray menu → App.
#[derive(Debug, Clone)]
pub enum Command {
    SelectProfile(String),
    ToggleOnOff,
    Quit,
}

/// Events from tray subscription → App.
#[derive(Debug, Clone)]
pub enum Event {
    Ready(watch::Sender<TrayState>),
    Command(Command),
}

/// State pushed from App → Tray for menu updates.
#[derive(Debug, Clone)]
pub struct TrayState {
    pub profile_names: Vec<String>,
    pub active_profile: String,
    pub is_connected: bool,
    pub is_on: bool,
}

impl Default for TrayState {
    fn default() -> Self {
        Self {
            profile_names: Vec::new(),
            active_profile: String::new(),
            is_connected: false,
            is_on: true,
        }
    }
}

struct SliglightTray {
    state: TrayState,
    command_tx: mpsc::Sender<Command>,
}

impl Tray for SliglightTray {
    fn id(&self) -> String {
        "sliglight".into()
    }

    fn title(&self) -> String {
        "Sliglight".into()
    }

    fn icon_name(&self) -> String {
        if !self.state.is_connected {
            "audio-input-microphone-muted".into()
        } else if self.state.is_on {
            "audio-input-microphone".into()
        } else {
            "audio-input-microphone-muted".into()
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let mut items: Vec<ksni::MenuItem<Self>> = Vec::new();

        // Profile items
        for name in &self.state.profile_names {
            let n = name.clone();
            let is_active = *name == self.state.active_profile;
            items.push(
                StandardItem {
                    label: if is_active {
                        format!("* {name}")
                    } else {
                        name.clone()
                    },
                    activate: Box::new(move |tray: &mut Self| {
                        let _ = tray
                            .command_tx
                            .try_send(Command::SelectProfile(n.clone()));
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }

        // Separator (empty label item)
        items.push(
            StandardItem {
                label: "---".into(),
                enabled: false,
                ..Default::default()
            }
            .into(),
        );

        // On/Off toggle
        let toggle_label = if self.state.is_on {
            "Turn Off"
        } else {
            "Turn On"
        };
        items.push(
            StandardItem {
                label: toggle_label.into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.command_tx.try_send(Command::ToggleOnOff);
                }),
                ..Default::default()
            }
            .into(),
        );

        // Quit
        items.push(
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.command_tx.try_send(Command::Quit);
                }),
                ..Default::default()
            }
            .into(),
        );

        items
    }
}

pub fn subscription() -> Subscription<Event> {
    Subscription::run(tray_worker)
}

fn tray_worker() -> impl iced::futures::Stream<Item = Event> {
    iced::stream::channel(32, async move |mut output| {
        let (command_tx, mut command_rx) = mpsc::channel::<Command>(32);
        let (state_tx, mut state_rx) = watch::channel(TrayState::default());

        let _ = output.send(Event::Ready(state_tx)).await;

        let tray = SliglightTray {
            state: TrayState::default(),
            command_tx,
        };

        let handle = match tray.spawn().await {
            Ok(h) => h,
            Err(e) => {
                log::warn!("Failed to create tray: {e}");
                std::future::pending::<()>().await;
                return;
            }
        };

        // State updater task.
        let update_handle = handle.clone();
        tokio::spawn(async move {
            while state_rx.changed().await.is_ok() {
                let new_state = state_rx.borrow_and_update().clone();
                update_handle
                    .update(|tray: &mut SliglightTray| {
                        tray.state = new_state;
                    })
                    .await;
            }
        });

        // Forward commands from tray to iced.
        while let Some(cmd) = command_rx.recv().await {
            let _ = output.send(Event::Command(cmd)).await;
        }
    })
}
