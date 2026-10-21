#!/usr/bin/env python3
"""Fullscreen overlay while work is paused for idle."""

from __future__ import annotations

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import Gdk, Gio, GLib, Gtk  # noqa: E402


def read_status() -> str | None:
    try:
        bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    except Exception:
        return None
    for sig in ("(s)", "s"):
        try:
            reply = bus.call_sync(
                "org.tomatobar.App",
                "/org/tomatobar/App",
                "org.tomatobar.App1",
                "Status",
                None,
                GLib.VariantType.new(sig),
                Gio.DBusCallFlags.NONE,
                500,
                None,
            )
        except Exception:
            continue
        value = reply.unpack()
        if isinstance(value, tuple):
            return str(value[0]) if value else None
        return str(value)
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


def still_idle_paused(fields: dict[str, str]) -> bool:
    return fields.get("phase") == "work" and fields.get("idle_paused") == "true"


def main():
    app = Gtk.Application(application_id="com.tomatobar.IdleMask")

    def on_activate(application: Gtk.Application):
        win = Gtk.ApplicationWindow(
            application=application, title="TomatoBar Work Paused", decorated=False
        )
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
        title = Gtk.Label(label="Work paused", css_classes=["title"])
        time_l = Gtk.Label(label="--:--", css_classes=["time"])
        body_l = Gtk.Label(
            label="Timer waits until you are back.",
            css_classes=["body"],
        )
        box.append(title)
        box.append(time_l)
        box.append(body_l)
        win.set_child(box)

        def tick() -> bool:
            status = read_status()
            if status is None:
                return True
            fields = parse_status(status)
            if not still_idle_paused(fields):
                win.close()
                return False
            try:
                secs = int(fields.get("remaining", "0"))
            except ValueError:
                secs = 0
            time_l.set_label(format_mm_ss(secs))
            return True

        tick()
        GLib.timeout_add_seconds(1, tick)
        win.fullscreen()
        win.present()

    app.connect("activate", on_activate)
    app.run([])


if __name__ == "__main__":
    main()
