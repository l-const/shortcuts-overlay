.PHONY: build install clean package-deb package-rpm install-cargo-deb install-cargo-rpm

# Binary name
BINARY_NAME = shortcuts-overlay
INSTALL_PATH = /usr/bin/shortcuts-overlay
SERVICE_NAME = shortcuts-overlay.service
DESKTOP_FILE = shortcuts-overlay.desktop
DESKTOP_DIR = /usr/share/applications
AUTOSTART_DIR = /etc/xdg/autostart
CONFIG_DIR = /usr/share/shortcuts-overlay
ICON_FILE = logo.svg
ICON_DIR = /usr/share/icons/hicolor/scalable/apps

# Build the project in release mode
build:
	cargo build --release

# Install the binary to /usr/bin/shortcut-overlay
install: build
	@echo "Installing $(BINARY_NAME) to $(INSTALL_PATH)..."
	sudo install -Dm755 target/release/$(BINARY_NAME) $(INSTALL_PATH)
	@echo "Creating config directory..."
	# mkdir -p $(CONFIG_DIR)
	@echo "Creating default config file..."
	sudo cp overlay-config.toml $(CONFIG_DIR)/overlay-config.toml
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
	sudo mkdir -p $(ICON_DIR)
	sudo install -Dm644 $(ICON_FILE) $(ICON_DIR)/shortcuts-overlay.svg
	@if command -v gtk-update-icon-cache > /dev/null 2>&1; then \
		sudo gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true; \
	fi
	@echo "Icon installed!"

# Install desktop entry
install-desktop: install-icon
	@echo "Installing desktop entry..."
	# mkdir -p $(DESKTOP_DIR)
	sudo install -Dm644 $(DESKTOP_FILE) $(DESKTOP_DIR)/$(DESKTOP_FILE)
	@if command -v update-desktop-database > /dev/null 2>&1; then \
		sudo update-desktop-database $(DESKTOP_DIR); \
	fi
	@echo "Desktop entry installed!"
	@echo "The application should now appear in your application menu"

# https://wiki.archlinux.org/title/XDG_Autostart
# Install autostart entry - create symbolic link to the desktop entry
install-autostart: install-desktop
	@echo "Installing autostart entry..."
	# mkdir -p $(AUTOSTART_DIR)
	sudo ln -sf $(DESKTOP_DIR)/$(DESKTOP_FILE) $(AUTOSTART_DIR)/$(DESKTOP_FILE)
	@echo "Autostart entry installed!"
	@echo "The application will now start automatically on login"

# Uninstall autostart entry
uninstall-autostart:
	@echo "Removing autostart entry..."
	sudo rm -f $(AUTOSTART_DIR)/$(DESKTOP_FILE)
	@echo "Autostart entry removed!"

# Uninstall icon
uninstall-icon:
	@echo "Removing icon..."
	sudo rm -f $(ICON_DIR)/shortcuts-overlay.svg
	@if command -v gtk-update-icon-cache > /dev/null 2>&1; then \
		sudo gtk-update-icon-cache -f -t $(HOME)/.local/share/icons/hicolor 2>/dev/null || true; \
	fi
	@echo "Icon uninstalled!"

# Uninstall desktop entry
uninstall-desktop: uninstall-icon
	@echo "Removing desktop entry..."
	sudo rm -f $(DESKTOP_DIR)/$(DESKTOP_FILE)
	@if command -v update-desktop-database > /dev/null 2>&1; then \
		sudo update-desktop-database $(DESKTOP_DIR); \
	fi
	@echo "Desktop entry uninstalled!"

# Install cargo-deb if not already installed
install-cargo-deb:
	@if ! command -v cargo-deb > /dev/null 2>&1; then \
		echo "Installing cargo-deb..."; \
		cargo install cargo-deb; \
	else \
		echo "cargo-deb is already installed"; \
	fi

# Install cargo-generate-rpm if not already installed
install-cargo-rpm:
	@if ! command -v cargo-generate-rpm > /dev/null 2>&1; then \
		echo "Installing cargo-generate-rpm..."; \
		cargo install cargo-generate-rpm; \
	else \
		echo "cargo-generate-rpm is already installed"; \
	fi

# Build Debian package
package-deb: install-cargo-deb build
	@echo "Building Debian package..."
	cargo deb
	@echo "Debian package created in target/debian/"
	@ls -lh target/debian/*.deb

# Build RPM package
package-rpm: install-cargo-rpm build
	@echo "Building RPM package..."
	cargo generate-rpm
	@echo "RPM package created in target/generate-rpm/"
	@ls -lh target/generate-rpm/*.rpm

# Build both packages
package-all: package-deb package-rpm
	@echo "All packages built successfully!"

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
	@echo "  make install-cargo-deb   - Install cargo-deb tool"
	@echo "  make install-cargo-rpm   - Install cargo-generate-rpm tool"
	@echo "  make package-deb         - Build Debian package (.deb)"
	@echo "  make package-rpm         - Build RPM package (.rpm)"
	@echo "  make package-all         - Build both Debian and RPM packages"
	@echo "  make help                - Show this help message"
