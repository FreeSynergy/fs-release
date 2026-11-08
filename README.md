# fs-release

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

## Verifying our releases

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

## Status (2026-11-06)

First real run green on `FreeSynergy/fs-capnpc` (tag `v0.1.1-rc.5`, draft release, not public):
`.tar.gz` (Linux x86_64/aarch64, macOS universal), `.deb` (amd64/arm64), `.rpm` (x86_64/aarch64),
Arch `.pkg.tar.zst`, Windows `.msi` + `.zip`, SPDX SBOM, `SHA256SUMS.txt`. CI (`ci.yml`) green incl. MSRV.

Lessons fixed on the way: callers must grant `packages: write`; macOS runs bash 3.2 (no `mapfile`, empty
arrays under `set -u`); `cargo-deb`/`cargo-wix` need `authors`; `cargo-generate-rpm` needs an existing
metadata table; RPM versions use `~` for SemVer pre-releases.

Not yet exercised: OpenPGP signatures (no key yet), AppImage/`.exe`/`.dmg` (apps only), Flatpak,
container, Android/iOS (with the first Blinc app).

