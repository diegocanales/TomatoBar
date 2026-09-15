use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::config::{xdg_open, Config};

/// Looping tick sound. Never signals process groups (TomatoBar shares the
/// gnome-session PGID; `kill(-pgid, …)` can tear down the whole user session).
struct Ticking {
    stop: Arc<AtomicBool>,
    paplay: Arc<Mutex<Option<Child>>>,
    join: Option<thread::JoinHandle<()>>,
}

pub struct Audio {
    bundled_dir: PathBuf,
    ticking: Mutex<Option<Ticking>>,
}

impl Audio {
    pub fn new() -> Self {
        let bundled_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/sounds");
        let _ = fs::create_dir_all(Config::sounds_dir());
        for name in ["windup.wav", "ding.wav", "ticking.wav"] {
            let dest = Config::sounds_dir().join(name);
            if !dest.exists() {
                let src = bundled_dir.join(name);
                if src.exists() {
                    let _ = fs::copy(&src, &dest);
                }
            }
        }
        Self {
            bundled_dir,
            ticking: Mutex::new(None),
        }
    }

    fn resolve_sound(&self, stem: &str) -> Option<PathBuf> {
        let exts = ["wav", "mp3", "ogg", "flac"];
        let dirs = [Config::sounds_dir(), self.bundled_dir.clone()];
        for dir in dirs {
            for ext in exts {
                let path = dir.join(format!("{stem}.{ext}"));
                if path.is_file() {
                    return Some(path);
                }
            }
        }
        None
    }

    fn play_once(path: &Path, volume: f32) {
        if volume <= 0.0 {
            return;
        }
        let vol = ((volume.clamp(0.0, 2.0)) * 32768.0) as i32;
        let path = path.to_path_buf();
        // Wait in a helper thread so exited paplay is reaped (no zombies).
        thread::spawn(move || {
            if let Ok(mut child) = Command::new("paplay")
                .arg("--volume")
                .arg(vol.to_string())
                .arg(&path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                let _ = child.wait();
            }
        });
    }

    pub fn play_windup(&self, volume: f32) {
        if let Some(path) = self.resolve_sound("windup") {
            Self::play_once(&path, volume);
        }
    }

    pub fn play_ding(&self, volume: f32) {
        if let Some(path) = self.resolve_sound("ding") {
            Self::play_once(&path, volume);
        }
    }

    pub fn start_ticking(&self, volume: f32) {
        self.stop_ticking();
        if volume <= 0.0 {
            return;
        }
        let Some(path) = self.resolve_sound("ticking") else {
            return;
        };
        let vol = ((volume.clamp(0.0, 2.0)) * 32768.0) as i32;
        let stop = Arc::new(AtomicBool::new(false));
        let paplay = Arc::new(Mutex::new(None));
        let stop_t = stop.clone();
        let paplay_t = paplay.clone();
        let join = thread::spawn(move || {
            while !stop_t.load(Ordering::SeqCst) {
                let child = match Command::new("paplay")
                    .arg("--volume")
                    .arg(vol.to_string())
                    .arg(&path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Ok(c) => c,
                    Err(_) => break,
                };
                match paplay_t.lock() {
                    Ok(mut slot) => *slot = Some(child),
                    Err(_) => break,
                }

                loop {
                    if stop_t.load(Ordering::SeqCst) {
                        break;
                    }
                    let done = {
                        let mut slot = match paplay_t.lock() {
                            Ok(s) => s,
                            Err(_) => break,
                        };
                        match slot.as_mut() {
                            Some(c) => match c.try_wait() {
                                Ok(Some(_)) => {
                                    // Already reaped; drop the Child handle.
                                    let _ = slot.take();
                                    true
                                }
                                Ok(None) => false,
                                Err(_) => {
                                    let _ = slot.take();
                                    true
                                }
                            },
                            None => true,
                        }
                    };
                    if done {
                        break;
                    }
                    thread::sleep(Duration::from_millis(50));
                }

                // Still running (stop requested): kill and reap this paplay only.
                if let Ok(mut slot) = paplay_t.lock() {
                    if let Some(mut c) = slot.take() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                }
            }
        });
        if let Ok(mut g) = self.ticking.lock() {
            *g = Some(Ticking {
                stop,
                paplay,
                join: Some(join),
            });
        }
    }

    pub fn stop_ticking(&self) {
        let Some(mut ticking) = self.ticking.lock().ok().and_then(|mut g| g.take()) else {
            return;
        };
        ticking.stop.store(true, Ordering::SeqCst);
        // Kill only the tracked paplay PID (never a process group).
        if let Ok(mut slot) = ticking.paplay.lock() {
            if let Some(mut child) = slot.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        if let Some(join) = ticking.join.take() {
            let _ = join.join();
        }
    }

    pub fn open_sound_folder() {
        let dir = Config::sounds_dir();
        let _ = fs::create_dir_all(&dir);
        xdg_open(&dir);
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        self.stop_ticking();
    }
}
