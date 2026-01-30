.PHONY: build install clean

# Binary name
BINARY_NAME = shortcuts-overlay
INSTALL_PATH = /usr/bin/shortcuts-overlay
SERVICE_NAME = shortcuts-overlay.service
DESKTOP_FILE = shortcuts-overlay.desktop
DESKTOP_DIR = $(HOME)/.local/share/applications
AUTOSTART_DIR = $(HOME)/.config/autostart
CONFIG_DIR = $(HOME)/.config/shortcuts-overlay
ICON_FILE = logo.svg
ICON_DIR = $(HOME)/.local/share/icons/hicolor/scalable/apps

# Build the project in release mode
build:
	cargo build --release

# Install the binary to /usr/bin/shortcut-overlay
install: build
	@echo "Installing $(BINARY_NAME) to $(INSTALL_PATH)..."
	sudo install -Dm755 target/release/$(BINARY_NAME) $(INSTALL_PATH)
	@echo "Creating config directory..."
	mkdir -p $(CONFIG_DIR)
	@echo "Creating default config file..."
	cp overlay-config.toml $(CONFIG_DIR)/overlay-config.toml
	@echo "Installation complete!"
	@echo "You can now run: shortcuts-overlay"
	@echo ""
	@echo "To install as a systemd service: make install-service"
	@echo "To install desktop entry: make install-desktop"

# Clean the target directory
clean:
	@echo "Cleaning target directory..."
	rm -rf target/
	@echo "Clean complete!"

# Uninstall the binary
uninstall:
	@echo "Removing $(INSTALL_PATH)..."
	sudo rm -f $(INSTALL_PATH)
	@echo "Uninstall complete!"


# Install icon
install-icon:
	@echo "Installing icon..."
	mkdir -p $(ICON_DIR)
	install -Dm644 $(ICON_FILE) $(ICON_DIR)/shortcuts-overlay.svg
	@if command -v gtk-update-icon-cache > /dev/null 2>&1; then \
		gtk-update-icon-cache -f -t $(HOME)/.local/share/icons/hicolor 2>/dev/null || true; \
	fi
	@echo "Icon installed!"

# Install desktop entry
install-desktop: install-icon
	@echo "Installing desktop entry..."
	mkdir -p $(DESKTOP_DIR)
	install -Dm644 $(DESKTOP_FILE) $(DESKTOP_DIR)/$(DESKTOP_FILE)
	@if command -v update-desktop-database > /dev/null 2>&1; then \
		update-desktop-database $(DESKTOP_DIR); \
	fi
	@echo "Desktop entry installed!"
	@echo "The application should now appear in your application menu"

# https://wiki.archlinux.org/title/XDG_Autostart
# Install autostart entry - create symbolic link to the desktop entry
install-autostart: install-desktop
	@echo "Installing autostart entry..."
	mkdir -p $(AUTOSTART_DIR)
	ln -sf $(DESKTOP_DIR)/$(DESKTOP_FILE) $(AUTOSTART_DIR)/$(DESKTOP_FILE)
	@echo "Autostart entry installed!"
	@echo "The application will now start automatically on login"

# Uninstall autostart entry
uninstall-autostart:
	@echo "Removing autostart entry..."
	rm -f $(AUTOSTART_DIR)/$(DESKTOP_FILE)
	@echo "Autostart entry removed!"

# Uninstall icon
uninstall-icon:
	@echo "Removing icon..."
	rm -f $(ICON_DIR)/shortcuts-overlay.svg
	@if command -v gtk-update-icon-cache > /dev/null 2>&1; then \
		gtk-update-icon-cache -f -t $(HOME)/.local/share/icons/hicolor 2>/dev/null || true; \
	fi
	@echo "Icon uninstalled!"

# Uninstall desktop entry
uninstall-desktop: uninstall-icon
	@echo "Removing desktop entry..."
	rm -f $(DESKTOP_DIR)/$(DESKTOP_FILE)
	@if command -v update-desktop-database > /dev/null 2>&1; then \
		update-desktop-database $(DESKTOP_DIR); \
	fi
	@echo "Desktop entry uninstalled!"

# Help target
help:
	@echo "Available targets:"
	@echo "  make build               - Build the project in release mode"
	@echo "  make install             - Build and install to /usr/bin/shortcuts-overlay"
	@echo "  make clean               - Remove the target/ directory"
	@echo "  make uninstall           - Remove the installed binary"
	@echo "  make install-service     - Install systemd user service"
	@echo "  make uninstall-service   - Remove systemd user service"
	@echo "  make install-icon        - Install application icon"
	@echo "  make uninstall-icon      - Remove application icon"
	@echo "  make install-desktop     - Install desktop entry (includes icon)"
	@echo "  make uninstall-desktop   - Remove desktop entry (includes icon)"
	@echo "  make install-autostart   - Install autostart entry (creates symlink)"
	@echo "  make uninstall-autostart - Remove autostart entry"
	@echo "  make help                - Show this help message"
