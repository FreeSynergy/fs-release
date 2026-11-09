# Formats, tools and licences

Rule (ADR 0015/0016): everything we ship is free; tools we run in CI may have any OSS licence, nothing proprietary
is ever part of a release. File names: `<name>-<version>-<os>-<arch>.<ext>`; deb, rpm and Arch packages keep the
naming their package managers expect. `.deb` and `.rpm` are built **per Cargo package that provides a binary** (a repo with an app
package and a server package gets two of each; see [usage.md](usage.md)); every other format is one file with all binaries.

| Platform | Format | Built with | Tool licence | Default for |
|----------|--------|-----------|--------------|-------------|
| Linux x86_64, aarch64 (native runners, glibc of Ubuntu 24.04) | `.tar.gz` | tar (reproducible flags) | GPL-3.0 (system tool) | all |
| | `.deb` | cargo-deb | MIT | all |
| | `.rpm` | cargo-generate-rpm | MIT | all |
| | AppImage | appimagetool 1.9.1 (pinned by SHA-256) | MIT | `app` |
| | Arch `.pkg.tar.zst` | makepkg in an `archlinux` container (x86_64 only; built around our tar.gz) | GPL-2.0-or-later | all |
| | Flatpak (optional) | flatpak-builder in the flathub-infra container | LGPL-2.1-or-later | opt-in; CLI tools may skip |
| Windows x64 (aarch64: zip only) | `.zip` portable | 7-Zip | LGPL-2.1 | all |
| | `.msi` | cargo-wix + WiX Toolset 3 | MIT OR Apache-2.0; MS-RL | all |
| | `-setup.exe` | NSIS `makensis`, our `installer.nsi` | zlib/libpng | `app` |
| macOS universal | `.tar.gz` | lipo + ad-hoc `codesign -s -` | Apple system tools on the runner | `cli`, `daemon` |
| | `.dmg` | hdiutil (app bundle generated) | Apple system tool on the runner | `app` |
| Android | APK, F-Droid | placeholder; F-Droid builds from source | see [fdroid.md](fdroid.md) | opt-in |
| iOS | `.app`, build only | placeholder | | opt-in |
| Container (optional) | OCI archive; push opt-in | docker buildx | Apache-2.0 | opt-in |
| All | SBOM `*.spdx.json` (SPDX 2.3) | cargo-sbom | MIT | always |
| All | `SHA256SUMS.txt`, `.asc` signatures, public key | sha256sum, GnuPG, dpkg-sig, rpmsign | GPL | always |
| All | SLSA provenance attestation | actions/attest-build-provenance | MIT | always |

CI helpers: Rust toolchain, clippy, rustfmt (MIT OR Apache-2.0), cargo-deny (MIT OR Apache-2.0), cargo-binstall (GPL-3.0, only
installs the tools above), Swatinem/rust-cache (MIT), actionlint (MIT), shellcheck (GPL-3.0; lint only). Open questions: cargo-packager
(MIT OR Apache-2.0) could replace our NSIS script and AppImage code later; we chose the small tools above because they need no config in the app repo.

## Free versus paid, and why macOS and Windows stay unsigned

- **Apple**: Developer ID signing and notarisation need the paid Apple Developer Program (USD 99 per year). Apple would be the only
  one earning money from our free software, so macOS builds are **not signed or notarised**. We do ad-hoc sign (`codesign -s -`,
  no account) because Apple Silicon refuses to run unsigned code at all. Users do a one-time "Open Anyway"
  (System Settings > Privacy & Security); the text ships in the download as `OPEN-ANYWAY.txt` and in the release notes.
  A donated Apple account would be fine; adding signing is then a single step in `package-macos`.
- **Windows**: Authenticode certificates cost money. SignPath Foundation signs OSS projects for free; to be asked of the owner
  on 2026-11-13. Until then `.msi`/`.exe` are unsigned (SmartScreen shows "More info > Run anyway").
- **Linux**: no payment needed. OpenPGP-signed packages and checksums are the trust path on every platform.
- **iOS**: only built, never distributed (no Apple account). **Android**: via F-Droid, which signs the apps itself.
- **GitHub Actions minutes** are free for public repositories, including the arm64 Linux and Windows runners.

## Install test (smoke)

Before the release draft exists, every package is installed on the system it is meant for and every binary is started once
(`--version`, must exit 0; input `smoke-args`): `.deb` in `debian:13` and `ubuntu:24.04` containers (x86_64 and aarch64),
Windows `.zip`, `.msi` (`msiexec /qn`) and `-setup.exe` (`/S`) on the Windows runner, each followed by the uninstall.
A failure stops the run. Not covered yet: `.rpm` (no Fedora container test), Arch, AppImage, dmg.
