# SmartExplorer

[![CI](https://github.com/ElHoussineDarrazi/SmartExplorer/actions/workflows/ci.yml/badge.svg)](https://github.com/ElHoussineDarrazi/SmartExplorer/actions/workflows/ci.yml)
[![Build and Release](https://github.com/ElHoussineDarrazi/SmartExplorer/actions/workflows/release.yml/badge.svg)](https://github.com/ElHoussineDarrazi/SmartExplorer/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://rust-lang.org)

A cross-platform file manager (Windows, Linux, macOS) written in Rust, with a 100 % Rust UI built on [egui/eframe](https://github.com/emilk/egui) — no system dependencies for the UI.

**Features**: tabbed navigation, image thumbnails, Grid/Icons view (with PDF previews on macOS), SSH/SFTP and FTP remote connections, encrypted credential storage (XChaCha20-Poly1305 + Argon2id), Computer Properties window (OS, CPU, GPU, RAM, storage), native system trash (Windows Recycle Bin / macOS & Linux Trash).

## Table of contents

- [Installation](#installation)
- [Building from source](#building-from-source)
- [Continuous integration & releases](#continuous-integration--releases)
- [Contributing](#contributing)
- [Credential security](#credential-security)
- [License](#license)

---

## Installation

Download the installer for your platform from the [Releases](https://github.com/ElHoussineDarrazi/SmartExplorer/releases) page :

| Platform | File |
|---|---|
| 🍎 macOS | `SmartExplorer-macos.dmg` |
| 🪟 Windows | `SmartExplorer-windows-setup.exe` |
| 🐧 Linux (Debian/Ubuntu) | `smartexplorer_<version>_amd64.deb` |

### 🍎 macOS — Gatekeeper workaround

The app is **not signed with an Apple Developer ID nor notarized** (a paid account is required). On first launch, macOS therefore shows:

> “SmartExplorer” Not Opened — Apple could not verify that …

This is **expected**: the code is safe, macOS simply cannot verify its origin. To launch the app, choose **one** of these methods:

**Method 1 — Right-click (simplest)**
1. In Finder, open `/Applications`.
2. **Right-click** **SmartExplorer** → **Open**.
3. Confirm **Open** in the dialog that appears (unlike the initial block, this one does offer “Open”).

**Method 2 — System Settings**
1. Try opening SmartExplorer once (it fails, which is expected).
2. Open **System Settings → Privacy & Security**.
3. At the bottom, click **“Open Anyway”** next to SmartExplorer.

**Method 3 — Terminal (one-time)**
```bash
xattr -dr com.apple.quarantine /Applications/SmartExplorer.app
```
The app will then launch normally, like any native app.

> These settings are only needed on the **first launch**; once bypassed, SmartExplorer starts directly.

### 🪟 Windows

Run `SmartExplorer-windows-setup.exe` and follow the wizard. If SmartScreen appears (unsigned binary), click **More info** then **Run anyway**.

### 🐧 Linux

```bash
sudo apt install ./smartexplorer_<version>_amd64.deb
```
Then launch **SmartExplorer** from your applications menu, or `smart_explorer` in a terminal.

---

## Building from source

Prerequisites: [Rust](https://rustup.rs) (the `cargo` tool).

```bash
git clone https://github.com/ElHoussineDarrazi/SmartExplorer.git
cd SmartExplorer
cargo build --release
```

### System dependencies per platform

- **Linux**: `libssh2-1-dev` (+ `pkg-config`) —
  `sudo apt install libssh2-1-dev pkg-config`
- **macOS**: `brew install libssh2`
- **Windows**: none — `ssh2` is built with vendored OpenSSL (`openssl-src`), which requires a full Perl (Strawberry Perl) if you reproduce the build manually.

The release binary is located in `target/release/`:

| Platform | Binary |
|---|---|
| macOS | `smart_explorer` (or `SmartExplorer.app` via `installer/macos/package.sh`) |
| Linux | `smart_explorer` |
| Windows | `smart_explorer.exe` |

> Note: `scripts/package_macos.sh` generates the `.icns` icon from `assets/icons/AppIcon.png` (requires `sips`, provided by macOS).

---

## Continuous integration & releases

Every pushed **tag** (`v*`) triggers the [`.github/workflows/release.yml`](.github/workflows/release.yml) workflow, which builds the app on all three platforms and automatically publishes the installers (`.dmg`, `.deb`, `setup.exe`) to the corresponding GitHub Release.

To cut a release:

```bash
# 1. Update `version` in Cargo.toml, then:
git commit -am "release: vX.Y.Z"

# 2. Create the tag and push it (CI publishes the installers):
git tag -fa vX.Y.Z -m "SmartExplorer vX.Y.Z"
git push origin main vX.Y.Z
```

---

## Credential security

SSH/FTP server passwords are **never stored in plaintext**: they are encrypted locally with XChaCha20-Poly1305, the key being derived from a master password via Argon2id (random salt per entry). Nothing is sent over the network — everything stays on your machine.

To report a vulnerability, follow [SECURITY.md](SECURITY.md) (private report, no public issue).

---

## Contributing

Contributions are welcome — code, documentation, translations, or ideas! 🙌

**New here?** Here is the typical path for a first contribution:

1. **Fork** the repo (the *Fork* button, top right).
2. Clone **your** fork and create a branch:

   ```bash
   git clone https://github.com/<your-username>/SmartExplorer.git
   cd SmartExplorer
   git checkout -b feature/my-feature
   ```

3. Edit the code, then check quality (the same checks as CI):

   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```

4. Push and open a **Pull Request**.

The details (code style, commit convention, tests) are in **[CONTRIBUTING.md](CONTRIBUTING.md)**. By participating, you agree to the **[Code of Conduct](CODE_OF_CONDUCT.md)**.

Every Pull Request is automatically validated by [CI](.github/workflows/ci.yml): formatting, strict lints, tests, and a build on all 3 platforms.

**Ideas to improve**: check the issues labeled [`good first issue`](https://github.com/ElHoussineDarrazi/SmartExplorer/labels/good%20first%20issue) to get started gently.

---

## License

Distributed under the **[MIT](LICENSE)** license — reuse, modify, and redistribute freely, including in commercial projects.
