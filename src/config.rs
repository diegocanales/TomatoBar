use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub const MAX_INTERVAL_SECS: u32 = 120 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StartWith {
    #[default]
    Work,
    Rest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StopAfter {
    #[default]
    Disabled,
    Work,
    Rest,
    LongRest,
}

#[derive(Debug, Clone)]
pub struct Preset {
    pub work_mins: u32,
    pub short_rest_mins: u32,
    pub long_rest_mins: u32,
    pub works_in_set: u32,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            work_mins: 25,
            short_rest_mins: 5,
            long_rest_mins: 15,
            works_in_set: 4,
        }
    }
}

impl Preset {
    pub fn clamp(mut self) -> Self {
        self.work_mins = self.work_mins.clamp(1, 120);
        self.short_rest_mins = self.short_rest_mins.clamp(1, 120);
        self.long_rest_mins = self.long_rest_mins.clamp(1, 120);
        self.works_in_set = self.works_in_set.clamp(1, 10);
        self
    }

    pub fn work_secs(&self) -> u32 {
        self.work_mins * 60
    }
    pub fn short_rest_secs(&self) -> u32 {
        self.short_rest_mins * 60
    }
    pub fn long_rest_secs(&self) -> u32 {
        self.long_rest_mins * 60
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub current_preset: usize,
    pub presets: [Preset; 4],
    pub start_with: StartWith,
    pub stop_after: StopAfter,
    pub start_timer_on_launch: bool,
    pub show_timer_in_icon: bool,
    pub toggle_dnd: bool,
    pub show_fullscreen_mask: bool,
    /// After a rest, wait until the user is active before starting the next work.
    pub hold_work_until_active: bool,
    pub autostart: bool,
    pub windup_volume: f32,
    pub ding_volume: f32,
    pub ticking_volume: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            current_preset: 0,
            presets: [
                Preset::default(),
                Preset::default(),
                Preset::default(),
                Preset::default(),
            ],
            start_with: StartWith::Work,
            stop_after: StopAfter::Disabled,
            start_timer_on_launch: false,
            show_timer_in_icon: true,
            toggle_dnd: false,
            show_fullscreen_mask: false,
            hold_work_until_active: false,
            autostart: false,
            windup_volume: 1.0,
            ding_volume: 1.0,
            ticking_volume: 1.0,
        }
    }
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
}

#[allow(dead_code)]
fn json_bool(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

fn parse_bool(s: &str) -> bool {
    matches!(s.trim(), "true" | "1" | "yes")
}

/// Minimal JSON helpers for our fixed schema (avoids serde_json dependency).
fn extract_number(text: &str, key: &str) -> Option<f64> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let after = &text[i + pat.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    if rest.starts_with('"') {
        return None;
    }
    let end = rest
        .find(|c: char| c == ',' || c == '}' || c == '\n')
        .unwrap_or(rest.len());
    rest[..end].trim().parse().ok()
}

fn extract_bool(text: &str, key: &str) -> Option<bool> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let after = &text[i + pat.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    let end = rest
        .find(|c: char| c == ',' || c == '}' || c == '\n')
        .unwrap_or(rest.len());
    Some(parse_bool(rest[..end].trim()))
}

fn extract_string<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let after = &text[i + pat.len()..];
    let colon = after.find(':')?;
    let mut rest = after[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    rest = &rest[1..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

impl Config {
    pub fn config_path() -> PathBuf {
        home().join(".config/tomatobar/config.json")
    }

    pub fn data_dir() -> PathBuf {
        home().join(".local/share/tomatobar")
    }

    pub fn sounds_dir() -> PathBuf {
        Self::data_dir().join("sounds")
    }

    pub fn events_log_path() -> PathBuf {
        Self::data_dir().join("events.jsonl")
    }

    pub fn autostart_desktop_path() -> PathBuf {
        home().join(".config/autostart/tomatobar.desktop")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let mut cfg = Self::default();
        let Ok(text) = fs::read_to_string(&path) else {
            return cfg;
        };

        if let Some(v) = extract_number(&text, "current_preset") {
            cfg.current_preset = (v as usize).min(3);
        }
        for (i, key_base) in ["work_mins", "short_rest_mins", "long_rest_mins", "works_in_set"]
            .iter()
            .enumerate()
        {
            // Prefer values inside the selected preset object by scanning all occurrences.
            let values: Vec<f64> = text
                .match_indices(&format!("\"{key_base}\""))
                .filter_map(|(idx, _)| {
                    let after = &text[idx..];
                    extract_number(after, key_base)
                })
                .collect();
            for (pi, val) in values.into_iter().take(4).enumerate() {
                match i {
                    0 => cfg.presets[pi].work_mins = val as u32,
                    1 => cfg.presets[pi].short_rest_mins = val as u32,
                    2 => cfg.presets[pi].long_rest_mins = val as u32,
                    3 => cfg.presets[pi].works_in_set = val as u32,
                    _ => {}
                }
            }
        }
        if let Some(s) = extract_string(&text, "start_with") {
            cfg.start_with = if s == "rest" {
                StartWith::Rest
            } else {
                StartWith::Work
            };
        }
        if let Some(s) = extract_string(&text, "stop_after") {
            cfg.stop_after = match s {
                "work" => StopAfter::Work,
                "rest" => StopAfter::Rest,
                "long_rest" => StopAfter::LongRest,
                _ => StopAfter::Disabled,
            };
        }
        if let Some(v) = extract_bool(&text, "start_timer_on_launch") {
            cfg.start_timer_on_launch = v;
        }
        if let Some(v) = extract_bool(&text, "show_timer_in_icon") {
            cfg.show_timer_in_icon = v;
        }
        if let Some(v) = extract_bool(&text, "toggle_dnd") {
            cfg.toggle_dnd = v;
        }
        if let Some(v) = extract_bool(&text, "show_fullscreen_mask") {
            cfg.show_fullscreen_mask = v;
        }
        if let Some(v) = extract_bool(&text, "hold_work_until_active") {
            cfg.hold_work_until_active = v;
        }
        if let Some(v) = extract_bool(&text, "autostart") {
            cfg.autostart = v;
        }
        if let Some(v) = extract_number(&text, "windup_volume") {
            cfg.windup_volume = v as f32;
        }
        if let Some(v) = extract_number(&text, "ding_volume") {
            cfg.ding_volume = v as f32;
        }
        if let Some(v) = extract_number(&text, "ticking_volume") {
            cfg.ticking_volume = v as f32;
        }
        cfg.sanitize();
        cfg
    }

    #[allow(dead_code)]
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut cfg = self.clone();
        cfg.sanitize();
        let mut presets = String::new();
        for (i, p) in cfg.presets.iter().enumerate() {
            if i > 0 {
                presets.push(',');
            }
            presets.push_str(&format!(
                "{{\"work_mins\":{},\"short_rest_mins\":{},\"long_rest_mins\":{},\"works_in_set\":{}}}",
                p.work_mins, p.short_rest_mins, p.long_rest_mins, p.works_in_set
            ));
        }
        let start_with = match cfg.start_with {
            StartWith::Work => "work",
            StartWith::Rest => "rest",
        };
        let stop_after = match cfg.stop_after {
            StopAfter::Disabled => "disabled",
            StopAfter::Work => "work",
            StopAfter::Rest => "rest",
            StopAfter::LongRest => "long_rest",
        };
        let text = format!(
            "{{\n  \"current_preset\": {},\n  \"presets\": [{}],\n  \"start_with\": \"{}\",\n  \"stop_after\": \"{}\",\n  \"start_timer_on_launch\": {},\n  \"show_timer_in_icon\": {},\n  \"toggle_dnd\": {},\n  \"show_fullscreen_mask\": {},\n  \"hold_work_until_active\": {},\n  \"autostart\": {},\n  \"windup_volume\": {},\n  \"ding_volume\": {},\n  \"ticking_volume\": {}\n}}\n",
            cfg.current_preset,
            presets,
            start_with,
            stop_after,
            json_bool(cfg.start_timer_on_launch),
            json_bool(cfg.show_timer_in_icon),
            json_bool(cfg.toggle_dnd),
            json_bool(cfg.show_fullscreen_mask),
            json_bool(cfg.hold_work_until_active),
            json_bool(cfg.autostart),
            cfg.windup_volume,
            cfg.ding_volume,
            cfg.ticking_volume
        );
        fs::write(path, text).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn sanitize(&mut self) {
        if self.current_preset > 3 {
            self.current_preset = 0;
        }
        for p in &mut self.presets {
            *p = p.clone().clamp();
        }
        self.windup_volume = self.windup_volume.clamp(0.0, 2.0);
        self.ding_volume = self.ding_volume.clamp(0.0, 2.0);
        self.ticking_volume = self.ticking_volume.clamp(0.0, 2.0);
    }

    pub fn preset(&self) -> &Preset {
        &self.presets[self.current_preset]
    }
}

pub fn xdg_open(path: &std::path::Path) {
    let _ = Command::new("xdg-open").arg(path).spawn();
}
