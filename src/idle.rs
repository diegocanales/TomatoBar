use zbus::proxy;

/// Input within this window counts as the user being back at the machine.
const ACTIVE_UNDER_MS: u64 = 3_000;

#[proxy(
    interface = "org.gnome.Mutter.IdleMonitor",
    default_service = "org.gnome.Mutter.IdleMonitor",
    default_path = "/org/gnome/Mutter/IdleMonitor/Core"
)]
trait IdleMonitor {
    fn get_idletime(&self) -> zbus::Result<u64>;
}

/// Session-bus watcher for Mutter idle time. A failed read counts as active
/// so a missing idle monitor cannot freeze the timer.
pub struct ActivityWatch {
    conn: Option<zbus::Connection>,
}

impl ActivityWatch {
    pub fn new() -> Self {
        Self { conn: None }
    }

    pub async fn user_is_active(&mut self) -> bool {
        if self.conn.is_none() {
            match zbus::Connection::session().await {
                Ok(conn) => self.conn = Some(conn),
                Err(_) => return true,
            }
        }
        let Some(conn) = self.conn.as_ref() else {
            return true;
        };
        let Ok(proxy) = IdleMonitorProxy::new(conn).await else {
            self.conn = None;
            return true;
        };
        match proxy.get_idletime().await {
            Ok(ms) => ms < ACTIVE_UNDER_MS,
            Err(_) => {
                self.conn = None;
                true
            }
        }
    }
}
