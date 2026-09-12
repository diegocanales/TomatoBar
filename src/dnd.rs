use std::process::Command;

/// Toggle GNOME notification banners as a pragmatic Do Not Disturb.
/// When `enabled`, banners are hidden (show-banners=false).
pub fn set_dnd(enabled: bool) {
    let value = if enabled { "false" } else { "true" };
    let _ = Command::new("gsettings")
        .args([
            "set",
            "org.gnome.desktop.notifications",
            "show-banners",
            value,
        ])
        .status();
}
