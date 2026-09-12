mod audio;
mod autostart;
mod config;
mod dbus_api;
mod dnd;
mod event_log;
mod icon;
mod notify;
mod timer;
mod tray;
mod ui_bridge;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, Mutex};

use audio::Audio;
use config::Config;
use dbus_api::{serve_dbus, AppCommand, Shared};
use timer::{Phase, Timer, TimerEvent};
use tray::TomatoTray;

fn print_help() {
    eprintln!(
        "TomatoBar — Pomodoro for the GNOME top bar\n\n\
Usage:\n\
  tomatobar                 Run the tray app\n\
  tomatobar start           Toggle start/stop (requires running app)\n\
  tomatobar pause           Pause/resume\n\
  tomatobar skip            Skip interval\n\
  tomatobar restart         Restart current work/rest interval\n\
  tomatobar add-minute      Add one minute\n\
  tomatobar stop            Stop to idle\n\
  tomatobar status          Print status\n"
    );
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    if let Some(cmd) = args.next() {
        match cmd.as_str() {
            "-h" | "--help" | "help" => {
                print_help();
                return;
            }
            "-V" | "--version" | "version" => {
                println!("tomatobar {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            name => {
                if let Err(e) = dbus_api::call_command(name).await {
                    eprintln!("{e}\nIs TomatoBar running? Try: make run");
                    std::process::exit(1);
                }
                return;
            }
        }
    }

    if let Err(e) = run_daemon().await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run_daemon() -> Result<(), String> {
    let config = Config::load();
    let _ = std::fs::create_dir_all(Config::data_dir());
    let _ = std::fs::create_dir_all(Config::sounds_dir());

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<AppCommand>();
    let timer = Timer::new(config.clone());
    let shared = Arc::new(Shared {
        timer: Mutex::new(timer),
        cmd_tx: cmd_tx.clone(),
    });

    let audio = Audio::new();

    // Refresh ~/.config/autostart entry (e.g. after install path / scripts change).
    {
        let exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.into_os_string().into_string().ok())
            .unwrap_or_else(|| "tomatobar".into());
        autostart::sync_autostart(config.autostart, &exe);
    }

    let tray = TomatoTray {
        shared: shared.clone(),
    };
    // AppIndicator may not be ready yet at session login; wait for the watcher.
    let handle = ksni::TrayMethods::assume_sni_available(tray, true)
        .spawn()
        .await
        .map_err(|e| format!("failed to show tray icon (is AppIndicator enabled?): {e}"))?;

    if let Err(e) = serve_dbus(shared.clone()).await {
        eprintln!("warning: D-Bus service not available ({e}); CLI control disabled");
    }

    if config.start_timer_on_launch {
        let _ = cmd_tx.send(AppCommand::StartStop);
    }

    let (skip_tx, mut skip_rx) = mpsc::unbounded_channel::<()>();

    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    ticker.tick().await;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                audio.stop_ticking();
                dnd::set_dnd(false);
                break;
            }
            _ = wait_terminate_signal() => {
                audio.stop_ticking();
                dnd::set_dnd(false);
                break;
            }
            _ = ticker.tick() => {
                let (events, before, after, cfg, paused) = {
                    let mut t = shared.timer.lock().await;
                    let before = t.phase;
                    let events = t.tick();
                    let after = t.phase;
                    for ev in &events {
                        if matches!(ev, TimerEvent::EnteredWork | TimerEvent::EnteredShortRest | TimerEvent::EnteredLongRest | TimerEvent::EnteredIdle) {
                            event_log::append_transition(before, after, *ev);
                        }
                    }
                    (events, before, after, t.config.clone(), t.paused)
                };
                apply_side_effects(&events, before, after, &cfg, paused, &audio, &cmd_tx, &skip_tx);
                handle.update(|_| {}).await;
            }
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    AppCommand::Quit => {
                        audio.stop_ticking();
                        dnd::set_dnd(false);
                        break;
                    }
                    AppCommand::OpenSettings => {
                        ui_bridge::open_settings(cmd_tx.clone());
                    }
                    AppCommand::OpenSoundFolder => {
                        Audio::open_sound_folder();
                    }
                    other => {
                        let (events, before, after, cfg, paused) = {
                            let mut t = shared.timer.lock().await;
                            let before = t.phase;
                            let events = match other {
                                AppCommand::StartStop => t.start_stop(),
                                AppCommand::Stop => t.stop(),
                                AppCommand::PauseResume => {
                                    t.pause_resume();
                                    let cfg = t.config.clone();
                                    if cfg.toggle_dnd {
                                        dnd::set_dnd(t.phase == Phase::Work && !t.paused);
                                    }
                                    if t.paused {
                                        audio.stop_ticking();
                                    } else if t.phase == Phase::Work {
                                        audio.start_ticking(cfg.ticking_volume);
                                    }
                                    vec![]
                                }
                                AppCommand::Skip => t.skip(),
                                AppCommand::AddMinute => {
                                    t.add_minute();
                                    vec![]
                                }
                                AppCommand::Restart => {
                                    t.restart_current();
                                    let cfg = t.config.clone();
                                    if t.phase == Phase::Work {
                                        audio.start_ticking(cfg.ticking_volume);
                                    }
                                    if cfg.toggle_dnd {
                                        dnd::set_dnd(t.phase == Phase::Work && !t.paused);
                                    }
                                    vec![]
                                }
                                AppCommand::ReloadConfig(cfg) => {
                                    let exe = std::env::current_exe()
                                        .ok()
                                        .and_then(|p| p.into_os_string().into_string().ok())
                                        .unwrap_or_else(|| "tomatobar".into());
                                    autostart::sync_autostart(cfg.autostart, &exe);
                                    t.reload_config(cfg);
                                    vec![]
                                }
                                _ => vec![],
                            };
                            let after = t.phase;
                            for ev in &events {
                                event_log::append_transition(before, after, *ev);
                            }
                            (events, before, after, t.config.clone(), t.paused)
                        };
                        apply_side_effects(&events, before, after, &cfg, paused, &audio, &cmd_tx, &skip_tx);
                        handle.update(|_| {}).await;
                    }
                }
            }
            Some(()) = skip_rx.recv() => {
                let _ = cmd_tx.send(AppCommand::Skip);
            }
        }
    }

    Ok(())
}

fn apply_side_effects(
    events: &[TimerEvent],
    from: Phase,
    _to: Phase,
    config: &Config,
    paused: bool,
    audio: &Audio,
    cmd_tx: &mpsc::UnboundedSender<AppCommand>,
    skip_tx: &mpsc::UnboundedSender<()>,
) {
    for ev in events {
        match ev {
            TimerEvent::EnteredWork => {
                if matches!(from, Phase::ShortRest | Phase::LongRest) {
                    notify::notify_break_over();
                }
                audio.play_windup(config.windup_volume);
                if !paused {
                    audio.start_ticking(config.ticking_volume);
                }
                if config.toggle_dnd {
                    dnd::set_dnd(true);
                }
            }
            TimerEvent::EnteredShortRest | TimerEvent::EnteredLongRest => {
                audio.stop_ticking();
                if matches!(from, Phase::Work) {
                    audio.play_ding(config.ding_volume);
                }
                if config.toggle_dnd {
                    dnd::set_dnd(false);
                }
                let long = matches!(ev, TimerEvent::EnteredLongRest);
                notify::show_rest_notification(long, skip_tx.clone());
                if config.show_fullscreen_mask {
                    let body = if long {
                        "Long break — you've earned it.".into()
                    } else {
                        "Short break — stretch for a few minutes.".into()
                    };
                    ui_bridge::show_mask(body, cmd_tx.clone());
                }
            }
            TimerEvent::EnteredIdle => {
                audio.stop_ticking();
                if config.toggle_dnd {
                    dnd::set_dnd(false);
                }
            }
            TimerEvent::Skipped => {}
        }
    }
}

async fn wait_terminate_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut sig) = signal(SignalKind::terminate()) {
            sig.recv().await;
            return;
        }
    }
    std::future::pending::<()>().await
}
