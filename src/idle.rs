use zbus::proxy;

/// Input within this window counts as the user being back at the machine.
pub const ACTIVE_UNDER_MS: u64 = 3_000;
/// Work pauses after this much idle time when the setting is on.
pub const WORK_IDLE_PAUSE_MS: u64 = 120_000;

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

    pub async fn idle_ms(&mut self) -> Option<u64> {
        if self.conn.is_none() {
            match zbus::Connection::session().await {
                Ok(conn) => self.conn = Some(conn),
                Err(_) => return None,
            }
        }
        let Some(conn) = self.conn.as_ref() else {
            return None;
        };
        let Ok(proxy) = IdleMonitorProxy::new(conn).await else {
            self.conn = None;
            return None;
        };
        match proxy.get_idletime().await {
            Ok(ms) => Some(ms),
            Err(_) => {
                self.conn = None;
                None
            }
        }
    }

}
