use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};
use zbus::{connection::Builder, interface, Connection};

use crate::config::Config;
use crate::timer::Timer;

#[derive(Debug, Clone)]
pub enum AppCommand {
    StartStop,
    PauseResume,
    Skip,
    AddMinute,
    Restart,
    Stop,
    ReloadConfig(Config),
    OpenSettings,
    OpenSoundFolder,
    Quit,
}

pub struct Shared {
    pub timer: Mutex<Timer>,
    pub cmd_tx: mpsc::UnboundedSender<AppCommand>,
}

pub async fn serve_dbus(shared: Arc<Shared>) -> zbus::Result<Connection> {
    let iface = TomatoBarIface {
        shared: shared.clone(),
    };
    Builder::session()?
        .name("org.tomatobar.App")?
        .serve_at("/org/tomatobar/App", iface)?
        .build()
        .await
}

struct TomatoBarIface {
    shared: Arc<Shared>,
}

#[interface(name = "org.tomatobar.App1")]
impl TomatoBarIface {
    async fn start_stop(&self) {
        let _ = self.shared.cmd_tx.send(AppCommand::StartStop);
    }

    async fn pause_resume(&self) {
        let _ = self.shared.cmd_tx.send(AppCommand::PauseResume);
    }

    async fn skip(&self) {
        let _ = self.shared.cmd_tx.send(AppCommand::Skip);
    }

    async fn add_minute(&self) {
        let _ = self.shared.cmd_tx.send(AppCommand::AddMinute);
    }

    async fn restart(&self) {
        let _ = self.shared.cmd_tx.send(AppCommand::Restart);
    }

    async fn stop(&self) {
        let _ = self.shared.cmd_tx.send(AppCommand::Stop);
    }

    async fn status(&self) -> String {
        let t = self.shared.timer.lock().await;
        format!(
            "phase={} remaining={} paused={} work={}",
            t.phase.label(),
            t.remaining_secs,
            t.paused,
            t.current_work
        )
    }
}

pub async fn call_command(cmd: &str) -> Result<(), String> {
    let conn = Connection::session()
        .await
        .map_err(|e| format!("D-Bus session error: {e}"))?;
    let proxy = zbus::Proxy::new(
        &conn,
        "org.tomatobar.App",
        "/org/tomatobar/App",
        "org.tomatobar.App1",
    )
    .await
    .map_err(|e| format!("TomatoBar is not running ({e})"))?;

    match cmd {
        "start" | "start-stop" | "startstop" => {
            proxy
                .call_method("StartStop", &())
                .await
                .map_err(|e| e.to_string())?;
        }
        "pause" | "pause-resume" | "pauseresume" => {
            proxy
                .call_method("PauseResume", &())
                .await
                .map_err(|e| e.to_string())?;
        }
        "skip" => {
            proxy
                .call_method("Skip", &())
                .await
                .map_err(|e| e.to_string())?;
        }
        "add-minute" | "addminute" => {
            proxy
                .call_method("AddMinute", &())
                .await
                .map_err(|e| e.to_string())?;
        }
        "restart" => {
            proxy
                .call_method("Restart", &())
                .await
                .map_err(|e| e.to_string())?;
        }
        "stop" => {
            proxy
                .call_method("Stop", &())
                .await
                .map_err(|e| e.to_string())?;
        }
        "status" => {
            let reply = proxy
                .call_method("Status", &())
                .await
                .map_err(|e| e.to_string())?;
            let s: String = reply.body().deserialize().map_err(|e| e.to_string())?;
            println!("{s}");
        }
        other => return Err(format!("unknown command: {other}")),
    }
    Ok(())
}
