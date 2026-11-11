# FreeSynergy Release — one way to ship, free of charge

**This is a building block. It is why you can check that a download really is ours.**

FreeSynergy Release holds the shared build and release recipes for all FreeSynergy repositories. One recipe checks the code, one packages it, signs it and describes it. Every program gets the same treatment, so we do not maintain twenty different pipelines.

What is different here: everything is free of charge for you and for us. A release comes with a list of checksums and an OpenPGP signature, a software bill of materials and build provenance, so you can see what is inside and who built it. We say openly where we cannot do something yet: macOS and Windows builds are not signed by the platform vendors, and the repo explains how to open them anyway.

## What it can do today

No release tag yet. The workflows are used by the repos through a caller file.

- CI: formatting, lints, tests, our own code-rules checker and a licence policy check.
- Packages: Linux (tar.gz, deb, rpm, AppImage, Arch, optional Flatpak), Windows (zip, msi, exe), macOS (tar.gz, dmg), optional container image.
- A software bill of materials (SPDX), build provenance, checksums and OpenPGP signatures.
- A draft GitHub release, never published automatically.
- First real run was green on FreeSynergy/fs-capnpc as a draft release: tar.gz for Linux and macOS, deb, rpm, Arch, msi and zip, SBOM and checksums.

## Virtual sessions

Tests and spikes that drive a desktop (typing test, KWin scripts, fake input) run in a virtual session with its own runtime directory and private D-Bus, never in the real one. Source `scripts/virtual-session.sh`, start with `scripts/typing-test.sh` or the pattern in DESIGN.md, and call tools only through `vs_run`/`vs_dbus`/`vs_qdbus`/`vs_gdbus`: they refuse to run against the real session.

## What is not there yet

- Signing has not been shown in a real release run yet. AppImage, exe, dmg, Flatpak and containers have not been exercised either.
- Android and iOS are placeholders until the first Blinc app exists.
- macOS and Windows packages are unsigned by the vendors.
- A previous signing key was revoked on 2026-11-07, because an AI agent had created it and knew its passphrase. The current key and its fingerprint are in the repo. Do not trust the revoked one.

## Part of FreeSynergy

Release is the same pipeline for every repo. FreeSynergy is a family of programs that are free for everyone, forever, and that keep your data yours. The documentation, the manifesto and the overall plan live in [fs-documentation](https://github.com/FreeSynergy/fs-documentation); the reading version is at <https://freesynergy.github.io/fs-documentation/>. Start with the [manifesto](https://freesynergy.github.io/fs-documentation/manifesto/) if you want to know why we do this.


## For developers

- No Rust workspace: GitHub Actions workflows (`ci.yml`, `release.yml`), composite actions with shell and Python packaging scripts, and golden tests in `tests/`.
- Call it from your repo; see [docs/usage.md](docs/usage.md) and the example callers in `examples/callers/`. Signing: `docs/signing.md`; formats: `docs/formats.md`.
- Verify a release: import `release-signing-key.asc`, then `gpg --verify` the file and run `sha256sum -c SHA256SUMS.txt`.
- Design: [DESIGN.md](DESIGN.md).
- Licence: MIT OR Apache-2.0.

## Reference

Shared CI and release workflows for all FreeSynergy repositories: one way to build, check, package, sign and describe our software.

- **CI** (`ci.yml`): fmt, clippy, tests, ADR 0043 rules checker, cargo-deny licence policy.
- **Check** (`check.yml`): the strict push gate - fmt, clippy with `clippy::pedantic`, workspace tests, ADR 0043 rules checker; no secrets.
- **Release** (`release.yml`): Linux (tar.gz, deb, rpm, AppImage, Arch, optional Flatpak), Windows (zip, msi, exe), macOS universal (tar.gz, dmg), optional OCI image,
  SPDX SBOM, SLSA provenance, SHA256SUMS and OpenPGP signatures, and a **draft** GitHub release (never published automatically).
  Android and iOS are placeholders until the first Blinc app.

Everything is free of charge for users and for us (ADR 0015/0016). macOS and Windows builds are unsigned; see [docs/formats.md](docs/formats.md).
On macOS: open the app once, then System Settings > Privacy & Security > "Open Anyway" (also shipped as `OPEN-ANYWAY.txt`).

| Doc | Content |
|-----|---------|
| [docs/usage.md](docs/usage.md) | calling the workflows, inputs, example callers |
| [DESIGN.md](DESIGN.md) | pattern map of the packaging scripts, state |
| [docs/signing.md](docs/signing.md) | OpenPGP key, secrets, verification |
| [docs/formats.md](docs/formats.md) | formats, tools, licences, free vs paid |
| [docs/standards.md](docs/standards.md) | OpenPGP, SPDX, SLSA, SemVer, Reproducible Builds |
| [docs/fdroid.md](docs/fdroid.md) | what an Android app repo must provide |
| [examples/callers/](examples/callers/) | ready callers for fs-capnpc and fs-hub |

Licence: MIT OR Apache-2.0.

### Verifying our releases

All release files and packages are signed with OpenPGP (RFC 9580):

- Key: `FreeSynergy Release <support@freesynergy.net>`, Ed25519, valid until 2029-11-05
- Fingerprint: `16A4 A3EE 563A EF51 FCF0  C55D ED7B 57C4 A190 4342` (since 2026-11-07, valid until 2029-11-07)
- Signing subkey (used by the workflows, the primary key stays offline): `372A 583D 05BF 8362 604C  F25D E26A 2347 F2B1 C097` (valid until 2027-11-07). Secrets are set per repository with `scripts/set-signing-secrets.sh`.
- The earlier key `C40F 0FBA 5C4C F71D 7BEB  B5C6 5478 A473 8FFE EB69` must not be trusted: an AI agent created it and knew its passphrase. It was revoked on 2026-11-07; the revocation is in `revoked-release-key-C40F0FBA.asc` and on keys.openpgp.org.
- Public key: [`release-signing-key.asc`](release-signing-key.asc)

```sh
gpg --import release-signing-key.asc
gpg --verify capnp-compiler-<version>-linux-x86_64.tar.gz.asc capnp-compiler-<version>-linux-x86_64.tar.gz
sha256sum -c SHA256SUMS.txt
```

### Status (2026-11-06)

First real run green on `FreeSynergy/fs-capnpc` (draft release, not public; version numbers count `0.0.N` since the reset, ADR 0064):
`.tar.gz` (Linux x86_64/aarch64, macOS universal), `.deb` (amd64/arm64), `.rpm` (x86_64/aarch64),
Arch `.pkg.tar.zst`, Windows `.msi` + `.zip`, SPDX SBOM, `SHA256SUMS.txt`. CI (`ci.yml`) green incl. MSRV.

Lessons fixed on the way: callers must grant `packages: write`; macOS runs bash 3.2 (no `mapfile`, empty
arrays under `set -u`); `cargo-deb`/`cargo-wix` need `authors`; `cargo-generate-rpm` needs an existing
metadata table; RPM versions use `~` for SemVer pre-releases.

Not yet exercised: OpenPGP signatures (no key yet), AppImage/`.exe`/`.dmg` (apps only), Flatpak,
container, Android/iOS (with the first Blinc app).
