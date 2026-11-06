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
