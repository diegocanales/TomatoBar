use std::fs;
use std::path::PathBuf;

use crate::config::Config;

fn scripts_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("TOMATOBAR_SCRIPTS") {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            return Some(p);
        }
    }
    let installed = Config::data_dir().join("scripts");
    if installed.is_dir() {
        return Some(installed);
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts");
    if dev.is_dir() {
        return Some(dev);
    }
    None
}

fn exec_line(binary_path: &str) -> String {
    match scripts_dir() {
        Some(dir) => format!("env TOMATOBAR_SCRIPTS={} {binary_path}", dir.display()),
        None => binary_path.to_string(),
    }
}

fn desktop_entry(exec: &str) -> String {
    format!(
        "[Desktop Entry]\n\
Type=Application\n\
Name=TomatoBar\n\
Comment=Pomodoro timer for the GNOME top bar\n\
Exec={exec}\n\
Icon=tomatobar\n\
Terminal=false\n\
Categories=Utility;Clock;\n\
StartupNotify=false\n\
X-GNOME-Autostart-enabled=true\n"
    )
}

pub fn sync_autostart(enabled: bool, binary_path: &str) {
    let path = Config::autostart_desktop_path();
    if enabled {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, desktop_entry(&exec_line(binary_path)));
    } else if path.exists() {
        let _ = fs::remove_file(&path);
    }
}
