# fs-release

Shared CI and release workflows for all FreeSynergy repositories: one way to build, check, package, sign and describe our software.

- **CI** (`ci.yml`): fmt, clippy, tests, ADR 0043 rules checker, cargo-deny licence policy.
- **Release** (`release.yml`): Linux (tar.gz, deb, rpm, AppImage, Arch, optional Flatpak), Windows (zip, msi, exe), macOS universal (tar.gz, dmg), optional OCI image,
  SPDX SBOM, SLSA provenance, SHA256SUMS and OpenPGP signatures, and a **draft** GitHub release (never published automatically).
  Android and iOS are placeholders until the first Blinc app.

Everything is free of charge for users and for us (ADR 0015/0016). macOS and Windows builds are unsigned; see [docs/formats.md](docs/formats.md).
On macOS: open the app once, then System Settings > Privacy & Security > "Open Anyway" (also shipped as `OPEN-ANYWAY.txt`).

| Doc | Content |
|-----|---------|
| [docs/usage.md](docs/usage.md) | calling the workflows, inputs, example callers |
| [docs/signing.md](docs/signing.md) | OpenPGP key, secrets, verification |
| [docs/formats.md](docs/formats.md) | formats, tools, licences, free vs paid |
| [docs/standards.md](docs/standards.md) | OpenPGP, SPDX, SLSA, SemVer, Reproducible Builds |
| [docs/fdroid.md](docs/fdroid.md) | what an Android app repo must provide |
| [examples/callers/](examples/callers/) | ready callers for fs-capnpc and fs-hub |

Licence: MIT OR Apache-2.0.

## Status (2026-11-06)

First real run green on `FreeSynergy/fs-capnpc` (tag `v0.1.1-rc.5`, draft release, not public):
`.tar.gz` (Linux x86_64/aarch64, macOS universal), `.deb` (amd64/arm64), `.rpm` (x86_64/aarch64),
Arch `.pkg.tar.zst`, Windows `.msi` + `.zip`, SPDX SBOM, `SHA256SUMS.txt`. CI (`ci.yml`) green incl. MSRV.

Lessons fixed on the way: callers must grant `packages: write`; macOS runs bash 3.2 (no `mapfile`, empty
arrays under `set -u`); `cargo-deb`/`cargo-wix` need `authors`; `cargo-generate-rpm` needs an existing
metadata table; RPM versions use `~` for SemVer pre-releases.

Not yet exercised: OpenPGP signatures (no key yet), AppImage/`.exe`/`.dmg` (apps only), Flatpak,
container, Android/iOS (with the first Blinc app).

