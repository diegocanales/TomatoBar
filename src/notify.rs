use std::process::Command;

use tokio::sync::mpsc::UnboundedSender;

pub fn notify_break_over() {
    std::thread::spawn(|| {
        let _ = Command::new("notify-send")
            .args(["-t", "6000", "Break is over", "Back to work."])
            .status();
    });
}

pub fn show_rest_notification(long: bool, skip_tx: UnboundedSender<()>) {
    let body = if long {
        "Long break — you've earned it."
    } else {
        "Short break — stretch for a few minutes."
    };
    // Actionable notifications via notify-send (GNOME): --action returns the key on stdout.
    std::thread::spawn(move || {
        let output = Command::new("notify-send")
            .args([
                "-t",
                "10000",
                "-a",
                "TomatoBar",
                "--action=skip=Skip",
                "Time's up",
                body,
            ])
            .output();
        if let Ok(out) = output {
            let key = String::from_utf8_lossy(&out.stdout);
            if key.trim() == "skip" {
                let _ = skip_tx.send(());
            }
        }
    });
}
