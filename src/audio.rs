use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::config::{xdg_open, Config};

pub struct Audio {
    bundled_dir: PathBuf,
    ticking: Mutex<Option<Child>>,
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
        let _ = Command::new("paplay")
            .arg("--volume")
            .arg(vol.to_string())
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
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
        // Own process group so Ctrl+C / stop can kill bash + paplay together.
        let mut cmd = Command::new("bash");
        cmd.arg("-c")
            .arg(format!(
                "while true; do paplay --volume {vol} '{}'; done",
                path.display()
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                // Become leader of a new process group (pgid == pid).
                if libc_setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = cmd.spawn().ok();
        if let Ok(mut g) = self.ticking.lock() {
            *g = child;
        }
    }

    pub fn stop_ticking(&self) {
        if let Ok(mut g) = self.ticking.lock() {
            if let Some(mut child) = g.take() {
                let pid = child.id();
                #[cfg(unix)]
                {
                    // Kill the whole process group (bash + paplay children).
                    let _ = Command::new("kill")
                        .args(["-TERM", &format!("-{pid}")])
                        .status();
                    let _ = Command::new("kill")
                        .args(["-KILL", &format!("-{pid}")])
                        .status();
                }
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        // Belt-and-suspenders: stop any orphaned paplay of our ticking file.
        let _ = Command::new("pkill")
            .args(["-f", "paplay .*ticking"])
            .status();
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

#[cfg(unix)]
unsafe fn libc_setpgid(pid: i32, pgid: i32) -> i32 {
    // Avoid adding a libc crate: call setpgid via libc linkage from std.
    extern "C" {
        fn setpgid(pid: i32, pgid: i32) -> i32;
    }
    setpgid(pid, pgid)
}
