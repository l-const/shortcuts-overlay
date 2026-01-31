# Packaging Guide

This document describes how to build distribution packages for `shortcuts-overlay` using `cargo-deb` (Debian/Ubuntu) and `cargo-generate-rpm` (Fedora/RHEL/openSUSE).

## Prerequisites

### For Debian/Ubuntu packages (.deb)
- Rust toolchain (1.70+)
- cargo-deb: `cargo install cargo-deb`

### For RPM packages (.rpm)
- Rust toolchain (1.70+)
- cargo-generate-rpm: `cargo install cargo-generate-rpm`

### Build Dependencies
These are needed to compile the application:
```bash
# Debian/Ubuntu
sudo apt-get install libwayland-dev libxkbcommon-dev libinput-dev libudev-dev

# Fedora/RHEL
sudo dnf install wayland-devel libxkbcommon-devel libinput-devel systemd-devel

# openSUSE
sudo zypper install wayland-devel libxkbcommon-devel libinput-devel systemd-devel
```

## Building Packages

### Quick Start with Makefile

The easiest way to build packages is using the provided Makefile:

```bash
# Build Debian package
make package-deb

# Build RPM package
make package-rpm

# Build both packages
make package-all
```

The Makefile will automatically install the required packaging tools if they're not present.

### Manual Building

#### Debian Package

1. Install cargo-deb:
   ```bash
   cargo install cargo-deb
   ```

2. Build the package:
   ```bash
   cargo deb
   ```

3. The package will be created in `target/debian/`:
   ```bash
   ls -lh target/debian/*.deb
   ```

4. Install the package:
   ```bash
   sudo dpkg -i target/debian/shortcuts-overlay_*.deb
   sudo apt-get install -f  # Install any missing dependencies
   ```

#### RPM Package

1. Install cargo-generate-rpm:
   ```bash
   cargo install cargo-generate-rpm
   ```

2. Build the package:
   ```bash
   cargo build --release
   cargo generate-rpm
   ```

3. The package will be created in `target/generate-rpm/`:
   ```bash
   ls -lh target/generate-rpm/*.rpm
   ```

4. Install the package:
   ```bash
   # Fedora/RHEL
   sudo dnf install target/generate-rpm/shortcuts-overlay-*.rpm
   
   # openSUSE
   sudo zypper install target/generate-rpm/shortcuts-overlay-*.rpm
   ```

## Package Contents

Both packages include the following files:

### Binaries
- `/usr/bin/shortcuts-overlay` - Main executable

### Application Files
- `/usr/share/applications/shortcuts-overlay.desktop` - Desktop entry
- `/usr/share/icons/hicolor/scalable/apps/shortcuts-overlay.svg` - Application icon

### Configuration
- `/usr/share/shortcuts-overlay/overlay-config.toml` - Default configuration template

### Documentation
- `/usr/share/doc/shortcuts-overlay/README.md` - Main documentation
- `/usr/share/doc/shortcuts-overlay/LICENSE` - License file

## Post-Installation

After installing the package, users need to:

1. **Add user to input group** (required for keyboard detection):
   ```bash
   sudo usermod -a -G input $USER
   ```

2. **Log out and log back in** for group changes to take effect

3. **Create user configuration** (optional):
   ```bash
   mkdir -p ~/.config/shortcuts-overlay
   cp /usr/share/shortcuts-overlay/overlay-config.toml ~/.config/shortcuts-overlay/
   ```

4. **Install desktop entry for current user** (optional, for autostart):
   ```bash
   mkdir -p ~/.local/share/applications
   cp /usr/share/applications/shortcuts-overlay.desktop ~/.local/share/applications/
   ```

## Package Dependencies

### Runtime Dependencies (Debian)
- libwayland-client0
- libxkbcommon0
- libinput10

### Runtime Dependencies (RPM)
- wayland
- libxkbcommon
- libinput

These are automatically installed by the package manager.

## Uninstalling

### Debian/Ubuntu
```bash
sudo apt-get remove shortcuts-overlay
# or to remove config files too:
sudo apt-get purge shortcuts-overlay
```

### Fedora/RHEL
```bash
sudo dnf remove shortcuts-overlay
```

### openSUSE
```bash
sudo zypper remove shortcuts-overlay
```

## Customizing Packages

### Updating Metadata

Edit `Cargo.toml` to customize package metadata:

```toml
[package.metadata.deb]
maintainer = "Your Name <your.email@example.com>"
copyright = "2024, Your Name <your.email@example.com>"
# ... other fields

[package.metadata.generate-rpm]
# ... RPM-specific fields
```

### Adding Files

To include additional files in packages, add them to the `assets` array in `Cargo.toml`:

```toml
[package.metadata.deb]
assets = [
    ["path/to/source", "path/in/package", "permissions"],
    # ...
]

[package.metadata.generate-rpm]
assets = [
    { source = "path/to/source", dest = "path/in/package", mode = "permissions" },
    # ...
]
```

## Troubleshooting

### cargo-deb not found
```bash
cargo install cargo-deb
```

### cargo-generate-rpm not found
```bash
cargo install cargo-generate-rpm
```

### Build fails due to missing dependencies
Install the build dependencies listed in the [Prerequisites](#prerequisites) section.

### Package installs but binary doesn't work
Ensure you have the runtime dependencies installed and that your user is in the `input` group.

### Icon not showing in application menu
Run the following to update caches:
```bash
# Update icon cache
gtk-update-icon-cache -f -t /usr/share/icons/hicolor

# Update desktop database
update-desktop-database /usr/share/applications
```

## CI/CD Integration

You can integrate package building into your CI/CD pipeline:

```yaml
# Example GitHub Actions workflow
name: Build Packages

on: [push, pull_request]

jobs:
  package:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libwayland-dev libxkbcommon-dev libinput-dev libudev-dev
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Build Debian package
        run: |
          cargo install cargo-deb
          cargo deb
      
      - name: Upload artifacts
        uses: actions/upload-artifact@v2
        with:
          name: packages
          path: target/debian/*.deb
```

## Distribution-Specific Notes

### Ubuntu/Debian
- Packages work on Ubuntu 20.04+ and Debian 11+
- Consider creating separate packages for different Ubuntu LTS versions if needed

### Fedora
- Tested on Fedora 38+
- Consider submitting to Fedora package repositories (COPR)

### RHEL/CentOS
- RHEL 9+ and compatible distributions
- May need to enable EPEL repository for some dependencies

### openSUSE
- Tested on openSUSE Tumbleweed and Leap 15.5+
- Consider submitting to openSUSE Build Service (OBS)

## Resources

- [cargo-deb documentation](https://github.com/kornelski/cargo-deb)
- [cargo-generate-rpm documentation](https://github.com/cat-in-136/cargo-generate-rpm)
- [Debian packaging guidelines](https://www.debian.org/doc/debian-policy/)
- [RPM packaging guidelines](https://rpm-packaging-guide.github.io/)