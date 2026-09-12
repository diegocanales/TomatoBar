use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;

use tokio::sync::mpsc::UnboundedSender;

use crate::dbus_api::AppCommand;

fn script(name: &str) -> PathBuf {
    if let Ok(dir) = std::env::var("TOMATOBAR_SCRIPTS") {
        let p = PathBuf::from(dir).join(name);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join(name)
}

pub fn open_settings(cmd_tx: UnboundedSender<AppCommand>) {
    thread::spawn(move || {
        let status = Command::new("python3")
            .arg(script("settings.py"))
            .env(
                "TOMATOBAR_BIN",
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.into_os_string().into_string().ok())
                    .unwrap_or_else(|| "tomatobar".into()),
            )
            .status();
        if status.map(|s| s.success()).unwrap_or(false) {
            let cfg = crate::config::Config::load();
            let _ = cmd_tx.send(AppCommand::ReloadConfig(cfg));
        }
    });
}

pub fn show_mask(body: String, cmd_tx: UnboundedSender<AppCommand>) {
    thread::spawn(move || {
        let output = Command::new("python3")
            .arg(script("mask.py"))
            .arg(&body)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        if let Ok(out) = output {
            if String::from_utf8_lossy(&out.stdout).contains("skip") {
                let _ = cmd_tx.send(AppCommand::Skip);
            }
        }
    });
}
