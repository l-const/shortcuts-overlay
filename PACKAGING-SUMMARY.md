# Packaging Summary

## Overview

The shortcuts-overlay project now supports automated packaging for both Debian-based (.deb) and RPM-based (.rpm) Linux distributions using:
- **cargo-deb**: For Debian/Ubuntu packages
- **cargo-generate-rpm**: For Fedora/RHEL/openSUSE packages

## Quick Commands

```bash
# Build Debian package
make package-deb

# Build RPM package
make package-rpm

# Build both packages
make package-all
```

## Package Configuration

All packaging configuration is defined in `Cargo.toml` under two metadata sections:

### 1. Debian Package (`[package.metadata.deb]`)

Includes:
- Package metadata (maintainer, description, dependencies)
- Asset installation paths
- Post-install/uninstall scripts for cache updates

### 2. RPM Package (`[package.metadata.generate-rpm]`)

Includes:
- RPM-specific metadata
- Asset installation with proper permissions
- Post-install script with user instructions
- Dependency specifications

## Files Packaged

Both package types include these files:

| Source File | Destination | Purpose |
|------------|-------------|---------|
| `target/release/shortcuts-overlay` | `/usr/bin/shortcuts-overlay` | Main executable |
| `logo.svg` | `/usr/share/icons/hicolor/scalable/apps/shortcuts-overlay.svg` | Application icon |
| `shortcuts-overlay.desktop` | `/usr/share/applications/shortcuts-overlay.desktop` | Desktop entry |
| `overlay-config.toml` | `/usr/share/shortcuts-overlay/overlay-config.toml` | Default config template |
| `README.md` | `/usr/share/doc/shortcuts-overlay/README.md` | Documentation |
| `LICENSE` | `/usr/share/doc/shortcuts-overlay/LICENSE` | Apache 2.0 license |

## Key Features

### Automatic Dependency Management
- **Debian**: Automatically detects and includes runtime dependencies (libwayland-client0, libxkbcommon0, libinput10)
- **RPM**: Specifies dependencies explicitly (wayland, libxkbcommon, libinput)

### Post-Installation Scripts
Both packages include scripts that:
- Update icon cache (`gtk-update-icon-cache`)
- Update desktop database (`update-desktop-database`)
- Display user instructions for adding to `input` group

### User Configuration
The default config is installed to `/usr/share/shortcuts-overlay/overlay-config.toml` as a template. Users can copy it to `~/.config/shortcuts-overlay/overlay-config.toml` to customize.

## Package Outputs

### Debian Package
- **Location**: `target/debian/shortcuts-overlay_0.1.0-1_amd64.deb`
- **Size**: ~2.0 MB
- **Architecture**: amd64
- **Format**: Debian binary package format

### RPM Package
- **Location**: `target/generate-rpm/shortcuts-overlay-0.1.0-1.x86_64.rpm`
- **Architecture**: x86_64
- **Format**: RPM package format

## Installation Commands

### Debian/Ubuntu
```bash
sudo dpkg -i target/debian/shortcuts-overlay_0.1.0-1_amd64.deb
sudo apt-get install -f  # Fix any dependency issues
```

### Fedora/RHEL
```bash
sudo dnf install target/generate-rpm/shortcuts-overlay-0.1.0-1.x86_64.rpm
```

### openSUSE
```bash
sudo zypper install target/generate-rpm/shortcuts-overlay-0.1.0-1.x86_64.rpm
```

## Post-Installation Steps

After package installation, users must:

1. Add themselves to the `input` group:
   ```bash
   sudo usermod -a -G input $USER
   ```

2. Log out and log back in for group changes to take effect

3. (Optional) Create user config:
   ```bash
   mkdir -p ~/.config/shortcuts-overlay
   cp /usr/share/shortcuts-overlay/overlay-config.toml ~/.config/shortcuts-overlay/
   ```

## CI/CD Integration

A GitHub Actions workflow (`.github/workflows/build-packages.yml`) has been added that:

1. **Builds packages** on every push/PR to main branch
2. **Tests installation** to verify package integrity
3. **Creates releases** when version tags are pushed (e.g., `v0.1.0`)
4. **Attaches packages** to GitHub releases automatically

### Workflow Jobs

- `build-deb`: Builds Debian package and uploads as artifact
- `build-rpm`: Builds RPM package and uploads as artifact
- `test-install-deb`: Tests Debian package installation
- `test-install-rpm`: Tests RPM package metadata
- `release`: Creates GitHub release with both packages (only on version tags)

## Documentation Files

Three documentation files have been created:

1. **PACKAGING.md**: Comprehensive packaging guide with detailed instructions
2. **QUICKSTART-PACKAGING.md**: Quick reference for building packages
3. **PACKAGING-SUMMARY.md**: This file - overview of packaging implementation

## Makefile Targets

New targets added to Makefile:

| Target | Description |
|--------|-------------|
| `make install-cargo-deb` | Install cargo-deb tool if not present |
| `make install-cargo-rpm` | Install cargo-generate-rpm tool if not present |
| `make package-deb` | Build Debian package (auto-installs cargo-deb) |
| `make package-rpm` | Build RPM package (auto-installs cargo-generate-rpm) |
| `make package-all` | Build both Debian and RPM packages |

## Dependencies

### Build-time Dependencies
- Rust toolchain (1.70+)
- libwayland-dev / wayland-devel
- libxkbcommon-dev / libxkbcommon-devel
- libinput-dev / libinput-devel
- libudev-dev / systemd-devel (for udev)

### Runtime Dependencies (auto-installed by packages)
- libwayland-client0 / wayland
- libxkbcommon0 / libxkbcommon
- libinput10 / libinput

## Testing Packages Locally

### Test Debian Package
```bash
# Build
make package-deb

# Inspect contents
dpkg-deb -c target/debian/*.deb

# Check package info
dpkg-deb -I target/debian/*.deb

# Install locally
sudo dpkg -i target/debian/*.deb
```

### Test RPM Package
```bash
# Build
make package-rpm

# Inspect contents
rpm -qlp target/generate-rpm/*.rpm

# Check package info
rpm -qip target/generate-rpm/*.rpm

# Install locally (on RPM-based system)
sudo dnf install target/generate-rpm/*.rpm
```

## Future Improvements

Potential enhancements for packaging:

1. **AUR Package**: Create PKGBUILD for Arch Linux users
2. **Flatpak/Snap**: Consider containerized formats for universal distribution
3. **Repository Hosting**: Set up APT/YUM repositories for easier installation
4. **Signing**: Sign packages with GPG keys for security
5. **Multi-architecture**: Build ARM64 packages for Pi and other ARM systems
6. **Systemd Service**: Add optional systemd user service for autostart
7. **AppImage**: Create AppImage for distribution-agnostic deployment

## Distribution Channels

Consider publishing packages to:

- **Debian/Ubuntu**: Launchpad PPA
- **Fedora**: COPR (Cool Other Package Repo)
- **openSUSE**: Open Build Service (OBS)
- **Arch Linux**: AUR (Arch User Repository)
- **GitHub Releases**: Automated via CI/CD workflow (already implemented)

## Troubleshooting

### Package build fails
- Ensure all build dependencies are installed
- Check Rust version is 1.70+
- Verify cargo-deb/cargo-generate-rpm are installed

### Binary doesn't work after installation
- Verify runtime dependencies are installed
- Check user is in `input` group: `groups`
- Verify access to `/dev/input/event*`: `ls -l /dev/input/event*`

### Icon/Desktop entry issues
- Update caches manually:
  ```bash
  gtk-update-icon-cache -f -t /usr/share/icons/hicolor
  update-desktop-database /usr/share/applications
  ```

## Maintainer Notes

When releasing a new version:

1. Update version in `Cargo.toml`
2. Update changelog/release notes
3. Commit changes
4. Create and push version tag:
   ```bash
   git tag v0.1.1
   git push origin v0.1.1
   ```
5. GitHub Actions will automatically build packages and create release

## License

This packaging configuration follows the same Apache 2.0 license as the main project.

## References

- [cargo-deb GitHub](https://github.com/kornelski/cargo-deb)
- [cargo-generate-rpm GitHub](https://github.com/cat-in-136/cargo-generate-rpm)
- [Debian Policy Manual](https://www.debian.org/doc/debian-policy/)
- [RPM Packaging Guide](https://rpm-packaging-guide.github.io/)
- [FHS - Filesystem Hierarchy Standard](https://refspecs.linuxfoundation.org/FHS_3.0/fhs/index.html)