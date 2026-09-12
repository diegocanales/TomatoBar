.PHONY: help setup run build check clean install uninstall

PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
APPDIR ?= $(HOME)/.local/share/applications
SHARE  ?= $(HOME)/.local/share/tomatobar
HICOLOR ?= $(HOME)/.local/share/icons/hicolor
ICON_SIZES ?= 48 128 256 512
SCRIPTDIR ?= $(SHARE)/scripts

help:
	@echo "TomatoBar (Linux) — available commands"
	@echo ""
	@echo "  make setup      Prepare the environment (check Rust, fetch deps)"
	@echo "  make run        Build and run the app (icon in the top bar)"
	@echo "  make build      Release build (target/release/tomatobar)"
	@echo "  make check      Type-check without running"
	@echo "  make install    Install binary, desktop entry, sounds, scripts"
	@echo "  make uninstall  Remove installed files"
	@echo "  make clean      Remove build artifacts"
	@echo ""
	@echo "CLI (while running): tomatobar status|start|pause|skip|restart|add-minute|stop"
	@echo ""
	@echo "If the icon does not appear, enable the GNOME extension"
	@echo "\"AppIndicator and KStatusNotifierItem Support\"."

setup:
	@command -v rustc >/dev/null || { echo "ERROR: rustc not found. Install Rust from https://rustup.rs"; exit 1; }
	@command -v cargo >/dev/null || { echo "ERROR: cargo not found. Install Rust from https://rustup.rs"; exit 1; }
	@echo "Rust OK: $$(rustc --version)"
	@echo "Recommended packages:"
	@echo "  sudo apt install python3-gi gir1.2-gtk-4.0 gir1.2-adw-1 pulseaudio-utils libnotify-bin"
	@cargo fetch
	@echo "Done. Run: make run"

run:
	@echo "Building and running... (Quit from the icon menu)"
	cargo run

build:
	cargo build --release
	@echo "Binary: target/release/tomatobar"

check:
	cargo check

clean:
	cargo clean

install: build
	mkdir -p "$(BINDIR)" "$(APPDIR)" "$(SHARE)/sounds" "$(SCRIPTDIR)"
	install -m 755 target/release/tomatobar "$(BINDIR)/tomatobar"
	cp assets/sounds/* "$(SHARE)/sounds/"
	@for sz in $(ICON_SIZES); do \
	  mkdir -p "$(HICOLOR)/$${sz}x$${sz}/apps"; \
	  cp "assets/icons/tomatobar-$${sz}.png" "$(HICOLOR)/$${sz}x$${sz}/apps/tomatobar.png"; \
	done
	# Remove non-standard / legacy paths that hid the app icon.
	rm -f "$(HICOLOR)/32x32/apps/tomatobar.png"
	rm -f "$(HICOLOR)/1024x1024/apps/tomatobar.png"
	cp scripts/*.py "$(SCRIPTDIR)/"
	chmod +x "$(SCRIPTDIR)"/*.py
	@printf '%s\n' \
	  '[Desktop Entry]' \
	  'Type=Application' \
	  'Name=TomatoBar' \
	  'Comment=Pomodoro timer for the GNOME top bar' \
	  'Exec=env TOMATOBAR_SCRIPTS=$(SCRIPTDIR) $(BINDIR)/tomatobar' \
	  'Icon=tomatobar' \
	  'Terminal=false' \
	  'Categories=Utility;Clock;' \
	  > "$(APPDIR)/tomatobar.desktop"
	-gtk-update-icon-cache -f -t "$(HICOLOR)" 2>/dev/null || true
	-update-desktop-database "$(APPDIR)" 2>/dev/null || true
	@echo "Installed to $(BINDIR)/tomatobar"

uninstall:
	rm -f "$(BINDIR)/tomatobar"
	rm -f "$(APPDIR)/tomatobar.desktop"
	@for sz in $(ICON_SIZES) 32 1024; do \
	  rm -f "$(HICOLOR)/$${sz}x$${sz}/apps/tomatobar.png"; \
	done
	rm -f "$(HOME)/.config/autostart/tomatobar.desktop"
	rm -rf "$(SCRIPTDIR)"
	@echo "Uninstalled (config/data under ~/.config/tomatobar and ~/.local/share/tomatobar kept)"
