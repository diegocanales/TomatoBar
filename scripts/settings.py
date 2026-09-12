#!/usr/bin/env python3
"""TomatoBar settings UI (GTK4 + libadwaita)."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Adw, Gtk  # noqa: E402

CONFIG = Path.home() / ".config/tomatobar/config.json"
AUTOSTART = Path.home() / ".config/autostart/tomatobar.desktop"


def default_config() -> dict:
    preset = {
        "work_mins": 25,
        "short_rest_mins": 5,
        "long_rest_mins": 15,
        "works_in_set": 4,
    }
    return {
        "current_preset": 0,
        "presets": [dict(preset) for _ in range(4)],
        "start_with": "work",
        "stop_after": "disabled",
        "start_timer_on_launch": False,
        "show_timer_in_icon": True,
        "toggle_dnd": False,
        "show_fullscreen_mask": False,
        "autostart": False,
        "windup_volume": 1.0,
        "ding_volume": 1.0,
        "ticking_volume": 1.0,
    }


def load_config() -> dict:
    if CONFIG.exists():
        try:
            data = json.loads(CONFIG.read_text())
            base = default_config()
            base.update(data)
            return base
        except Exception:
            pass
    return default_config()


def save_config(cfg: dict) -> None:
    CONFIG.parent.mkdir(parents=True, exist_ok=True)
    CONFIG.write_text(json.dumps(cfg, indent=2) + "\n")
    sync_autostart(bool(cfg.get("autostart")))


def sync_autostart(enabled: bool) -> None:
    if enabled:
        AUTOSTART.parent.mkdir(parents=True, exist_ok=True)
        exe = os.environ.get("TOMATOBAR_BIN", "tomatobar")
        AUTOSTART.write_text(
            "[Desktop Entry]\n"
            "Type=Application\n"
            "Name=TomatoBar\n"
            "Exec=" + exe + "\n"
            "Icon=tomatobar\n"
            "Terminal=false\n"
            "X-GNOME-Autostart-enabled=true\n"
        )
    elif AUTOSTART.exists():
        AUTOSTART.unlink()


class SettingsWindow(Adw.ApplicationWindow):
    def __init__(self, app: Adw.Application):
        super().__init__(
            application=app,
            title="TomatoBar Settings",
            default_width=440,
            default_height=560,
        )
        self.set_deletable(True)
        self.cfg = load_config()
        box = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL,
            spacing=12,
            margin_top=16,
            margin_bottom=16,
            margin_start=16,
            margin_end=16,
        )

        def add_spin(label: str, value: float, lo: float, hi: float) -> Gtk.SpinButton:
            row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            lab = Gtk.Label(label=label, hexpand=True, xalign=0)
            adj = Gtk.Adjustment(value=value, lower=lo, upper=hi, step_increment=1)
            spin = Gtk.SpinButton(adjustment=adj, digits=0)
            row.append(lab)
            row.append(spin)
            box.append(row)
            return spin

        def add_switch(label: str, active: bool) -> Gtk.Switch:
            row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            lab = Gtk.Label(label=label, hexpand=True, xalign=0)
            sw = Gtk.Switch(active=active, valign=Gtk.Align.CENTER)
            row.append(lab)
            row.append(sw)
            box.append(row)
            return sw

        def add_drop(label: str, options: list[str], selected: int) -> Gtk.DropDown:
            row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
            lab = Gtk.Label(label=label, hexpand=True, xalign=0)
            drop = Gtk.DropDown.new_from_strings(options)
            drop.set_selected(selected)
            row.append(lab)
            row.append(drop)
            box.append(row)
            return drop

        idx = int(self.cfg.get("current_preset", 0)) % 4
        p = self.cfg["presets"][idx]

        box.append(Gtk.Label(label="Intervals (current preset)", xalign=0, css_classes=["title-4"]))
        self.preset_drop = add_drop("Preset", ["Preset 1", "Preset 2", "Preset 3", "Preset 4"], idx)
        self.work = add_spin("Work (min)", p["work_mins"], 1, 120)
        self.short = add_spin("Short rest (min)", p["short_rest_mins"], 1, 120)
        self.long = add_spin("Long rest (min)", p["long_rest_mins"], 1, 120)
        self.works = add_spin("Works in set", p["works_in_set"], 1, 10)

        box.append(Gtk.Label(label="Behavior", xalign=0, css_classes=["title-4"]))
        sw_map = {"work": 0, "rest": 1}
        sa_map = {"disabled": 0, "work": 1, "rest": 2, "long_rest": 3}
        self.start_with = add_drop("Start with", ["Work", "Rest"], sw_map.get(self.cfg.get("start_with", "work"), 0))
        self.stop_after = add_drop("Stop after", ["Disabled", "Work", "Rest", "Long rest"], sa_map.get(self.cfg.get("stop_after", "disabled"), 0))
        self.start_on_launch = add_switch("Start timer on launch", self.cfg.get("start_timer_on_launch", False))
        self.show_timer = add_switch("Show timer next to icon", self.cfg.get("show_timer_in_icon", True))
        self.dnd = add_switch("Do Not Disturb during work", self.cfg.get("toggle_dnd", False))
        self.mask = add_switch("Fullscreen mask on rest", self.cfg.get("show_fullscreen_mask", False))
        self.autostart = add_switch("Launch at login", self.cfg.get("autostart", False))

        box.append(Gtk.Label(label="Volumes (0–200%)", xalign=0, css_classes=["title-4"]))
        self.windup = add_spin("Windup %", self.cfg.get("windup_volume", 1.0) * 100, 0, 200)
        self.ding = add_spin("Ding %", self.cfg.get("ding_volume", 1.0) * 100, 0, 200)
        self.ticking = add_spin("Ticking %", self.cfg.get("ticking_volume", 1.0) * 100, 0, 200)

        actions = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8, halign=Gtk.Align.END)
        close_btn = Gtk.Button(label="Close")
        close_btn.connect("clicked", lambda _b: self.close())
        save = Gtk.Button(label="Save", css_classes=["suggested-action"])
        save.connect("clicked", self.on_save)
        actions.append(close_btn)
        actions.append(save)
        box.append(actions)

        scroll = Gtk.ScrolledWindow(child=box, vexpand=True)

        # Adw CSD windows need a HeaderBar or the window chrome (incl. close) is missing.
        header = Adw.HeaderBar()
        header.set_title_widget(Gtk.Label(label="TomatoBar Settings"))
        toolbar = Adw.ToolbarView()
        toolbar.add_top_bar(header)
        toolbar.set_content(scroll)
        self.set_content(toolbar)

    def on_save(self, _btn):
        idx = int(self.preset_drop.get_selected())
        self.cfg["current_preset"] = idx
        self.cfg["presets"][idx] = {
            "work_mins": int(self.work.get_value()),
            "short_rest_mins": int(self.short.get_value()),
            "long_rest_mins": int(self.long.get_value()),
            "works_in_set": int(self.works.get_value()),
        }
        self.cfg["start_with"] = "work" if self.start_with.get_selected() == 0 else "rest"
        self.cfg["stop_after"] = ["disabled", "work", "rest", "long_rest"][self.stop_after.get_selected()]
        self.cfg["start_timer_on_launch"] = self.start_on_launch.get_active()
        self.cfg["show_timer_in_icon"] = self.show_timer.get_active()
        self.cfg["toggle_dnd"] = self.dnd.get_active()
        self.cfg["show_fullscreen_mask"] = self.mask.get_active()
        self.cfg["autostart"] = self.autostart.get_active()
        self.cfg["windup_volume"] = self.windup.get_value() / 100.0
        self.cfg["ding_volume"] = self.ding.get_value() / 100.0
        self.cfg["ticking_volume"] = self.ticking.get_value() / 100.0
        save_config(self.cfg)
        # Signal parent via exit code 0; Rust polls mtime / sends reload after process ends.
        self.close()


def main():
    Adw.init()
    app = Adw.Application(application_id="com.tomatobar.Settings")
    app.connect("activate", lambda a: SettingsWindow(a).present())
    sys.exit(app.run(sys.argv))


if __name__ == "__main__":
    main()
