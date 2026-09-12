#!/usr/bin/env python3
"""Fullscreen rest mask for TomatoBar."""

from __future__ import annotations

import sys

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import Gdk, Gtk  # noqa: E402


def main():
    body = sys.argv[1] if len(sys.argv) > 1 else "Take a short break."
    app = Gtk.Application(application_id="com.tomatobar.Mask")

    def on_activate(app: Gtk.Application):
        win = Gtk.ApplicationWindow(application=app, title="TomatoBar Rest", decorated=False)
        win.add_css_class("mask")
        css = Gtk.CssProvider()
        css.load_from_data(
            b"window.mask { background-color: rgba(0,0,0,0.82); }"
            b"label.title { color: white; font-size: 28pt; }"
            b"label.body { color: #ddd; font-size: 16pt; }"
        )
        display = Gdk.Display.get_default()
        if display:
            Gtk.StyleContext.add_provider_for_display(
                display, css, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
            )
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=24, valign=Gtk.Align.CENTER, halign=Gtk.Align.CENTER)
        title = Gtk.Label(label="Break time", css_classes=["title"])
        body_l = Gtk.Label(label=body, css_classes=["body"])
        skip = Gtk.Button(label="Skip break", css_classes=["suggested-action"])

        def on_skip(_):
            print("skip", flush=True)
            win.close()

        skip.connect("clicked", on_skip)
        box.append(title)
        box.append(body_l)
        box.append(skip)
        win.set_child(box)
        win.fullscreen()
        win.present()

    app.connect("activate", on_activate)
    app.run([])


if __name__ == "__main__":
    main()
