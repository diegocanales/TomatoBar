#!/usr/bin/env python3
"""Fullscreen rest mask for TomatoBar."""

from __future__ import annotations

import sys

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import Gdk, Gio, GLib, Gtk  # noqa: E402

REST_PHASES = {"short_rest", "long_rest"}


def read_status() -> str | None:
    try:
        bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        reply = bus.call_sync(
            "org.tomatobar.App",
            "/org/tomatobar/App",
            "org.tomatobar.App1",
            "Status",
            None,
            GLib.VariantType.new("(s)"),
            Gio.DBusCallFlags.NONE,
            500,
            None,
        )
        value = reply.unpack()
        if isinstance(value, tuple):
            return str(value[0]) if value else None
        return str(value)
    except Exception:
        return None


def parse_status(status: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    for part in status.split():
        if "=" not in part:
            continue
        key, val = part.split("=", 1)
        fields[key] = val
    return fields


def format_mm_ss(secs: int) -> str:
    secs = max(0, secs)
    return f"{secs // 60:02d}:{secs % 60:02d}"


def main():
    body = sys.argv[1] if len(sys.argv) > 1 else "Take a short break."
    app = Gtk.Application(application_id="com.tomatobar.Mask")

    def on_activate(application: Gtk.Application):
        win = Gtk.ApplicationWindow(application=application, title="TomatoBar Rest", decorated=False)
        win.add_css_class("mask")
        css = Gtk.CssProvider()
        css.load_from_data(
            b"window.mask { background-color: rgba(0,0,0,0.82); }"
            b"label.title { color: white; font-size: 28pt; }"
            b"label.time { color: white; font-size: 72pt; font-weight: bold; }"
            b"label.body { color: #ddd; font-size: 16pt; }"
        )
        display = Gdk.Display.get_default()
        if display:
            Gtk.StyleContext.add_provider_for_display(
                display, css, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
            )
        box = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL,
            spacing=24,
            valign=Gtk.Align.CENTER,
            halign=Gtk.Align.CENTER,
        )
        title = Gtk.Label(label="Break time", css_classes=["title"])
        time_l = Gtk.Label(label="--:--", css_classes=["time"])
        paused_l = Gtk.Label(label="", css_classes=["body"])
        body_l = Gtk.Label(label=body, css_classes=["body"])
        skip = Gtk.Button(label="Skip break", css_classes=["suggested-action"])

        def on_skip(_):
            print("skip", flush=True)
            win.close()

        def tick() -> bool:
            status = read_status()
            if status is None:
                return True
            fields = parse_status(status)
            phase = fields.get("phase", "")
            if phase not in REST_PHASES:
                win.close()
                return False
            try:
                secs = int(fields.get("remaining", "0"))
            except ValueError:
                secs = 0
            time_l.set_label(format_mm_ss(secs))
            paused_l.set_label("Paused" if fields.get("paused") == "true" else "")
            return True

        skip.connect("clicked", on_skip)
        box.append(title)
        box.append(time_l)
        box.append(paused_l)
        box.append(body_l)
        box.append(skip)
        win.set_child(box)
        tick()
        GLib.timeout_add_seconds(1, tick)
        win.fullscreen()
        win.present()

    app.connect("activate", on_activate)
    app.run([])


if __name__ == "__main__":
    main()
