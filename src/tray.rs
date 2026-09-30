use std::sync::Arc;

use crate::dbus_api::{AppCommand, Shared};
use crate::icon;
use crate::timer::{Phase, Timer};

pub struct TomatoTray {
    pub shared: Arc<Shared>,
}

impl TomatoTray {
    fn with_timer<R>(&self, f: impl FnOnce(&Timer) -> R) -> R {
        // ksni Tray methods are sync; use try_lock / blocking.
        loop {
            if let Ok(t) = self.shared.timer.try_lock() {
                return f(&t);
            }
            std::thread::yield_now();
        }
    }

    fn send(&self, cmd: AppCommand) {
        let _ = self.shared.cmd_tx.send(cmd);
    }
}

impl ksni::Tray for TomatoTray {
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }

    fn title(&self) -> String {
        self.with_timer(|t| match t.phase {
            Phase::Idle => "TomatoBar".into(),
            Phase::AwaitingUser => "Waiting for you".into(),
            _ => {
                let mut s = t.display_time();
                if t.paused {
                    s = format!("⏸ {s}");
                }
                if let Some(p) = t.work_progress_label() {
                    s = format!("{s} ({p})");
                }
                s
            }
        })
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.with_timer(|t| vec![icon::tray_icon(t)])
    }

    fn xayatana_label(&self) -> String {
        self.with_timer(icon::tray_label)
    }

    fn xayatana_label_guide(&self) -> String {
        self.with_timer(icon::tray_label_guide)
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        self.with_timer(|t| {
            let (title, description) = match t.phase {
                Phase::Idle => ("TomatoBar".into(), "Idle — Start from the menu".into()),
                Phase::Work => (
                    t.display_time(),
                    if t.paused {
                        "Work paused".into()
                    } else {
                        "Work interval".into()
                    },
                ),
                Phase::ShortRest => (t.display_time(), "Short rest".into()),
                Phase::LongRest => (t.display_time(), "Long rest".into()),
                Phase::AwaitingUser => (
                    "Waiting for you".into(),
                    "Rest over. Work starts when you are back.".into(),
                ),
            };
            ksni::ToolTip {
                title,
                description,
                ..Default::default()
            }
        })
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        let (phase, paused, remaining, progress) = self.with_timer(|t| {
            (
                t.phase,
                t.paused,
                t.display_time(),
                t.work_progress_label(),
            )
        });

        let mut items: Vec<ksni::MenuItem<Self>> = Vec::new();

        if phase == Phase::Idle {
            items.push(
                StandardItem {
                    label: "Start".into(),
                    icon_name: "media-playback-start".into(),
                    activate: Box::new(|this: &mut Self| this.send(AppCommand::StartStop)),
                    ..Default::default()
                }
                .into(),
            );
        } else if phase == Phase::AwaitingUser {
            items.push(
                StandardItem {
                    label: "Stop".into(),
                    icon_name: "media-playback-stop".into(),
                    activate: Box::new(|this: &mut Self| this.send(AppCommand::Stop)),
                    ..Default::default()
                }
                .into(),
            );
            items.push(
                StandardItem {
                    label: "Start work".into(),
                    icon_name: "media-skip-forward".into(),
                    activate: Box::new(|this: &mut Self| this.send(AppCommand::Skip)),
                    ..Default::default()
                }
                .into(),
            );
        } else {
            let stop_label = match phase {
                Phase::Work => format!("Stop work ({remaining})"),
                Phase::ShortRest => format!("Stop rest ({remaining})"),
                Phase::LongRest => format!("Stop long rest ({remaining})"),
                Phase::AwaitingUser | Phase::Idle => remaining,
            };
            items.push(
                StandardItem {
                    label: stop_label,
                    icon_name: "media-playback-stop".into(),
                    activate: Box::new(|this: &mut Self| this.send(AppCommand::Stop)),
                    ..Default::default()
                }
                .into(),
            );
            items.push(
                StandardItem {
                    label: if paused {
                        "Resume".into()
                    } else {
                        "Pause".into()
                    },
                    icon_name: if paused {
                        "media-playback-start".into()
                    } else {
                        "media-playback-pause".into()
                    },
                    activate: Box::new(|this: &mut Self| this.send(AppCommand::PauseResume)),
                    ..Default::default()
                }
                .into(),
            );
            items.push(
                StandardItem {
                    label: "Skip".into(),
                    icon_name: "media-skip-forward".into(),
                    activate: Box::new(|this: &mut Self| this.send(AppCommand::Skip)),
                    ..Default::default()
                }
                .into(),
            );
            items.push(
                StandardItem {
                    label: "+1 minute".into(),
                    icon_name: "list-add".into(),
                    activate: Box::new(|this: &mut Self| this.send(AppCommand::AddMinute)),
                    ..Default::default()
                }
                .into(),
            );
            items.push(
                StandardItem {
                    label: "Restart interval".into(),
                    icon_name: "view-refresh".into(),
                    activate: Box::new(|this: &mut Self| this.send(AppCommand::Restart)),
                    ..Default::default()
                }
                .into(),
            );
            if let Some(p) = progress {
                items.push(
                    StandardItem {
                        label: format!("Set progress: {p}"),
                        enabled: false,
                        ..Default::default()
                    }
                    .into(),
                );
            }
        }

        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: "Settings…".into(),
                icon_name: "preferences-system".into(),
                activate: Box::new(|this: &mut Self| this.send(AppCommand::OpenSettings)),
                ..Default::default()
            }
            .into(),
        );
        items.push(
            StandardItem {
                label: "Open sound folder".into(),
                icon_name: "folder".into(),
                activate: Box::new(|this: &mut Self| this.send(AppCommand::OpenSoundFolder)),
                ..Default::default()
            }
            .into(),
        );
        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|this: &mut Self| this.send(AppCommand::Quit)),
                ..Default::default()
            }
            .into(),
        );
        items
    }
}

