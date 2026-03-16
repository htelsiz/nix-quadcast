//! Listens for `org.freedesktop.ScreenSaver.ActiveChanged` DBus signals.
//!
//! Emits `Event::Locked` when the screen locks and `Event::Unlocked` when it unlocks.

use iced::futures::SinkExt;
use iced::Subscription;
use zbus::proxy;

#[derive(Debug, Clone)]
pub enum Event {
    Locked,
    Unlocked,
}

#[proxy(
    interface = "org.freedesktop.ScreenSaver",
    default_service = "org.freedesktop.ScreenSaver",
    default_path = "/org/freedesktop/ScreenSaver"
)]
trait ScreenSaver {
    #[zbus(signal)]
    fn active_changed(&self, active: bool) -> zbus::Result<()>;
}

pub fn subscription() -> Subscription<Event> {
    Subscription::run(screen_lock_worker)
}

fn screen_lock_worker() -> impl iced::futures::Stream<Item = Event> {
    iced::stream::channel(8, async move |mut output| {
        let Ok(conn) = zbus::Connection::session().await else {
            log::warn!("screen_lock: failed to connect to session bus");
            std::future::pending::<()>().await;
            return;
        };

        let Ok(proxy) = ScreenSaverProxy::new(&conn).await else {
            log::warn!("screen_lock: failed to create ScreenSaver proxy");
            std::future::pending::<()>().await;
            return;
        };

        let Ok(mut stream) = proxy.receive_active_changed().await else {
            log::warn!("screen_lock: failed to subscribe to ActiveChanged signal");
            std::future::pending::<()>().await;
            return;
        };

        use iced::futures::StreamExt;
        while let Some(signal) = stream.next().await {
            if let Ok(args) = signal.args() {
                let event = if args.active {
                    Event::Locked
                } else {
                    Event::Unlocked
                };
                let _ = output.send(event).await;
            }
        }
    })
}
