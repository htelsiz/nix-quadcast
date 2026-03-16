//! System tray icon via KDE StatusNotifierItem (ksni).
//!
//! Shows a tray icon with a menu for profile switching and app control.
//! Uses the app's own SVG rendered to ARGB32 pixmap for a custom icon.

use iced::futures::SinkExt;
use iced::Subscription;
use ksni::menu::StandardItem;
use ksni::{Icon, Tray, TrayMethods};
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

/// Render the embedded SVG to an ARGB32 pixmap for the tray icon.
fn render_tray_icon() -> Icon {
    let svg_data = include_bytes!("../../../resources/sliglight.svg");
    let size = 48i32;
    let tree = resvg::usvg::Tree::from_data(svg_data, &resvg::usvg::Options::default())
        .expect("embedded SVG must parse");
    let mut pixmap =
        tiny_skia::Pixmap::new(size as u32, size as u32).expect("pixmap allocation");
    let svg_size = tree.size();
    let scale = size as f32 / svg_size.width().max(svg_size.height());
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // tiny_skia gives us premultiplied RGBA. ksni wants ARGB32 in network byte order
    // (big-endian), so each pixel is [A, R, G, B].
    let rgba = pixmap.data();
    let mut argb = Vec::with_capacity(rgba.len());
    for pixel in rgba.chunks_exact(4) {
        let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);
        // Un-premultiply if alpha > 0.
        if a == 0 {
            argb.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let ur = ((r as u16 * 255) / a as u16).min(255) as u8;
            let ug = ((g as u16 * 255) / a as u16).min(255) as u8;
            let ub = ((b as u16 * 255) / a as u16).min(255) as u8;
            argb.extend_from_slice(&[a, ur, ug, ub]);
        }
    }

    Icon {
        width: size,
        height: size,
        data: argb,
    }
}

struct SliglightTray {
    state: TrayState,
    icon: Icon,
    command_tx: mpsc::Sender<Command>,
}

impl Tray for SliglightTray {
    fn id(&self) -> String {
        "sliglight".into()
    }

    fn title(&self) -> String {
        "Sliglight".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![self.icon.clone()]
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
            icon: render_tray_icon(),
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
