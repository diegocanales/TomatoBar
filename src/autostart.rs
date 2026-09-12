use std::fs;

use crate::config::Config;

fn desktop_entry(exec: &str, autostart_only: bool) -> String {
    let mut body = format!(
        "[Desktop Entry]\n\
Type=Application\n\
Name=TomatoBar\n\
Comment=Pomodoro timer for the GNOME top bar\n\
Exec={exec}\n\
Icon=tomatobar\n\
Terminal=false\n\
Categories=Utility;Clock;\n\
StartupNotify=false\n"
    );
    if autostart_only {
        body.push_str("X-GNOME-Autostart-enabled=true\n");
    }
    body
}

pub fn sync_autostart(enabled: bool, binary_path: &str) {
    let path = Config::autostart_desktop_path();
    if enabled {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, desktop_entry(binary_path, true));
    } else if path.exists() {
        let _ = fs::remove_file(&path);
    }
}
