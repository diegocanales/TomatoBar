use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Config;
use crate::timer::{Phase, TimerEvent};

pub fn append_transition(from: Phase, to: Phase, event: TimerEvent) {
    let path = Config::events_log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let event = match event {
        TimerEvent::EnteredWork => "entered_work",
        TimerEvent::EnteredShortRest => "entered_short_rest",
        TimerEvent::EnteredLongRest => "entered_long_rest",
        TimerEvent::EnteredIdle => "entered_idle",
        TimerEvent::Skipped => "skipped",
    };
    let line = format!(
        r#"{{"ts":{ts},"event":"{event}","from":"{}","to":"{}"}}"#,
        from.label(),
        to.label()
    );
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}
