# Quick Start: Building Packages

This is a quick reference for building distribution packages. For detailed instructions, see [PACKAGING.md](PACKAGING.md).

## TL;DR

```bash
# Build Debian package
make package-deb

# Build RPM package
make package-rpm

# Build both
make package-all
```

## Prerequisites

### Build Dependencies

**Debian/Ubuntu:**
```bash
sudo apt-get install libwayland-dev libxkbcommon-dev libinput-dev libudev-dev
```

**Fedora/RHEL:**
```bash
sudo dnf install wayland-devel libxkbcommon-devel libinput-devel systemd-devel
```

**openSUSE:**
```bash
sudo zypper install wayland-devel libxkbcommon-devel libinput-devel systemd-devel
```

### Packaging Tools

The Makefile will automatically install `cargo-deb` and `cargo-generate-rpm` if needed.

To install manually:
```bash
cargo install cargo-deb
cargo install cargo-generate-rpm
```

## Building Packages

### Debian Package (.deb)

```bash
# Using Makefile (recommended)
make package-deb

# Manual
cargo deb

# Output location
ls target/debian/*.deb
```

### RPM Package (.rpm)

```bash
# Using Makefile (recommended)
make package-rpm

# Manual
cargo build --release
cargo generate-rpm

# Output location
ls target/generate-rpm/*.rpm
```

## Installing Packages

### Debian/Ubuntu

```bash
sudo dpkg -i target/debian/shortcuts-overlay_*.deb
sudo apt-get install -f  # Fix any dependency issues
```

### Fedora/RHEL

```bash
sudo dnf install target/generate-rpm/shortcuts-overlay-*.rpm
```

### openSUSE

```bash
sudo zypper install target/generate-rpm/shortcuts-overlay-*.rpm
```

## Package Contents

Both packages include:

| File | Location |
|------|----------|
| Binary | `/usr/bin/shortcuts-overlay` |
| Desktop Entry | `/usr/share/applications/shortcuts-overlay.desktop` |
| Icon | `/usr/share/icons/hicolor/scalable/apps/shortcuts-overlay.svg` |
| Default Config | `/usr/share/shortcuts-overlay/overlay-config.toml` |
| Documentation | `/usr/share/doc/shortcuts-overlay/` |

## Post-Installation Steps

After installing the package:

1. **Add user to input group** (required):
   ```bash
   sudo usermod -a -G input $USER
   ```

2. **Log out and log back in** (required for group change)

3. **Create user config** (optional):
   ```bash
   mkdir -p ~/.config/shortcuts-overlay
   cp /usr/share/shortcuts-overlay/overlay-config.toml ~/.config/shortcuts-overlay/
   ```

4. **Run the application**:
   ```bash
   shortcuts-overlay
   ```
   Or launch from your application menu.

## Customizing Packages

Edit the `[package.metadata.deb]` and `[package.metadata.generate-rpm]` sections in `Cargo.toml` to customize:

- Maintainer information
- Package dependencies
- File installation paths
- Post-install/uninstall scripts

See [PACKAGING.md](PACKAGING.md) for detailed customization options.

## Troubleshooting

### Build fails with missing dependencies
Install the build dependencies listed above for your distribution.

### cargo-deb or cargo-generate-rpm not found
```bash
cargo install cargo-deb cargo-generate-rpm
```

### Package installs but binary doesn't work
1. Check runtime dependencies are installed
2. Verify user is in `input` group: `groups`
3. Check permissions on `/dev/input/event*`: `ls -l /dev/input/event*`

### Icon not showing
Update caches:
```bash
gtk-update-icon-cache -f -t /usr/share/icons/hicolor
update-desktop-database /usr/share/applications
```

## Uninstalling

**Debian/Ubuntu:**
```bash
sudo apt-get remove shortcuts-overlay
# or purge to remove config files
sudo apt-get purge shortcuts-overlay
```

**Fedora/RHEL:**
```bash
sudo dnf remove shortcuts-overlay
```

**openSUSE:**
```bash
sudo zypper remove shortcuts-overlay
```

## Distribution Channels

Consider distributing packages through:

- **Debian/Ubuntu**: Personal Package Archives (PPA)
- **Fedora**: COPR repositories
- **openSUSE**: Open Build Service (OBS)
- **Arch Linux**: AUR (PKGBUILD)
- **GitHub Releases**: Attach built packages to releases

## More Information

- [PACKAGING.md](PACKAGING.md) - Detailed packaging guide
- [cargo-deb](https://github.com/kornelski/cargo-deb) - Debian packaging tool
- [cargo-generate-rpm](https://github.com/cat-in-136/cargo-generate-rpm) - RPM packaging tool