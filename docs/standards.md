# Standards behind the release process (ADR 0014: standards first)

| Standard | Used for | Why |
|----------|----------|-----|
| **OpenPGP, RFC 9580** | detached `.asc` signatures, native deb/rpm package signatures | Open, no vendor or certificate authority, supported by every Linux package manager, free. The current RFC (2024) supersedes RFC 4880; Ed25519 is part of it. We use GnuPG, which reads and writes both. |
| **SPDX, ISO/IEC 5962:2021** | SBOM per release (`*.spdx.json`, version 2.3) | An ISO standard, machine readable, tooling for licence compliance. It lets users see exactly which crates and licences are inside, which also backs ADR 0015/0016. |
| **SLSA** (build provenance) | `actions/attest-build-provenance`, Sigstore-backed attestation | Proves that a file was built by this workflow from this commit. Checked with `gh attestation verify`. Complements the OpenPGP signature (who released it) with how it was built. |
| **SemVer 2.0.0** | tags `vX.Y.Z[-pre][+build]`, validated by the plan job | Predictable upgrades; the tag must equal the `Cargo.toml` version. Pre-releases become GitHub pre-releases; MSI uses the numeric part. |
| **Reproducible Builds** | `--locked`, `Cargo.lock`, tar `--sort/--owner/--mtime`, pinned tool versions | Anyone should be able to rebuild and get the same bytes. F-Droid requires it for Android; it also makes SLSA provenance meaningful. Not yet bit-exact everywhere (deb/rpm/AppImage/msi embed timestamps); tracked as future work. |

Other: the Cap'n Proto interface contract (ADR 0021) and AS2 data standards do not affect releases.
Draft-only releases follow the owner decision: public only when it helps.
