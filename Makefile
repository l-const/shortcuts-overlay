.PHONY: build install clean

# Binary name
BINARY_NAME = shortcuts-overlay
INSTALL_PATH = /usr/bin/shortcut-overlay

# Build the project in release mode
build:
	cargo build --release

# Install the binary to /usr/bin/shortcut-overlay
install: build
	@echo "Installing $(BINARY_NAME) to $(INSTALL_PATH)..."
	sudo install -Dm755 target/release/$(BINARY_NAME) $(INSTALL_PATH)
	@echo "Installation complete!"
	@echo "You can now run: shortcut-overlay"

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

# Help target
help:
	@echo "Available targets:"
	@echo "  make build     - Build the project in release mode"
	@echo "  make install   - Build and install to /usr/bin/shortcut-overlay"
	@echo "  make clean     - Remove the target/ directory"
	@echo "  make uninstall - Remove the installed binary"
	@echo "  make help      - Show this help message"
