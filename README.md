# TomatoBar (Linux)

Pomodoro timer for the Ubuntu/GNOME **top bar**. Practical port of the macOS TomatoBar fork.

## Features (v1.0)

- Work → short rest → (after N works) long rest cycle
- Countdown next to the tray icon via Ayatana label (idle / W / R / L / pause)
- Start / Stop / Pause / Skip / +1 minute
- 4 presets and configurable intervals (up to 120 minutes)
- Start with work or rest; stop-after options
- Sounds: windup, ding, ticking via PulseAudio (`paplay`)
- Desktop notifications with Skip on rest
- Optional GNOME Do Not Disturb during work
- Optional fullscreen rest mask (GTK)
- Autostart / launch at login
- JSONL event log
- CLI + D-Bus control

## Prerequisites (Ubuntu 24.04)

1. GNOME extension **AppIndicator and KStatusNotifierItem Support**
2. Runtime tools (usually preinstalled):

```bash
sudo apt install python3-gi python3-gi-cairo gir1.2-gtk-4.0 gir1.2-adw-1 pulseaudio-utils libnotify-bin
```

## Usage

```bash
cd tomatobar
make setup
make run
```

Install to `~/.local`:

```bash
make install
tomatobar
```

### Tray menu

Start, Stop, Pause/Resume, Skip, +1 minute, Settings…, Open sound folder, Quit.

### CLI (app must already be running)

```bash
tomatobar status
tomatobar start          # toggle start/stop
tomatobar pause
tomatobar skip
tomatobar add-minute
tomatobar stop
```

### Config and data

| Path | Purpose |
|------|---------|
| `~/.config/tomatobar/config.json` | Settings / presets / volumes |
| `~/.local/share/tomatobar/sounds/` | `windup` / `ding` / `ticking` (wav/mp3/ogg) |
| `~/.local/share/tomatobar/events.jsonl` | State transition log |

Settings UI is a GTK4/libadwaita window launched via `scripts/settings.py`.

## Out of scope (post-v1)

Global hotkeys (Wayland), `tomatobar://` URL scheme, Flatpak packaging.
