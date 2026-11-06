# Using fs-release

fs-release offers two reusable workflows. A repo adds two tiny caller files; everything else lives here.

| Workflow | Purpose |
|----------|---------|
| `.github/workflows/ci.yml` | fmt, clippy, tests, our coding rules (ADR 0043), licence check |
| `.github/workflows/release.yml` | plan, build per platform, SBOM, signatures, provenance, **draft** release |

Internal building blocks (`_release-*.yml`, `actions/*`) are not meant to be called directly.
All third-party actions are pinned to commit SHAs (the version is in a comment); Dependabot
or a manual bump keeps them current (ADR 0026).

## CI

```yaml
# .github/workflows/ci.yml
name: ci
on:
  push: { branches: [main] }
  pull_request:
permissions:
  contents: read
jobs:
  ci:
    uses: FreeSynergy/fs-release/.github/workflows/ci.yml@v1
    with:
      msrv: "1.85"          # optional
```

Inputs: `working-directory` (default `.`), `features` (`all` = `--all-features`, `default`, or a list),
`msrv` (extra `cargo check` on that toolchain), `apt-packages`, `runs-on`, `rules` (default true),
`rules-url`, `licences` (default true).

What runs, in order: `cargo fmt --check` (no `--all`, coding.md section 7), `cargo clippy --all-targets
--all-features -- -D warnings`, `cargo test --all-features`, `scripts/check_rust_rules.py` fetched from
`FreeSynergy/fs-documentation` (`main`), then `cargo deny check licenses bans sources` with
[`configs/deny.toml`](../configs/deny.toml) (allow-list MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, MPL-2.0 ...;
GPL/AGPL/LGPL/unknown are rejected) unless the repo has its own `deny.toml`. Advisories are reported as a warning only.
The rules checker is downloaded without a token, so `fs-documentation` must be readable (public) or `rules-url` must point elsewhere.

## Release

Push a tag `vX.Y.Z` (SemVer; a pre-release such as `v1.0.0-rc.1` is marked as pre-release). The tag version must equal the
`version` in `Cargo.toml`, otherwise the plan job fails. The result is a **draft** release; publishing is always a manual step.

```yaml
# .github/workflows/release.yml
name: release
on:
  push: { tags: ["v*"] }
permissions:
  contents: write
  id-token: write
  attestations: write
jobs:
  release:
    uses: FreeSynergy/fs-release/.github/workflows/release.yml@v1
    with:
      binaries: capnp-compiler
      kind: cli
      app-id: net.freesynergy.CapnpCompiler
    secrets:
      GPG_PRIVATE_KEY: ${{ secrets.GPG_PRIVATE_KEY }}
      GPG_PASSPHRASE: ${{ secrets.GPG_PASSPHRASE }}
```

The caller must grant the three permissions; a called workflow can never have more than its caller.
Full examples: [`examples/callers/`](../examples/callers/) (fs-capnpc as CLI, fs-hub as daemon with OCI image).

### Inputs

| Input | Default | Meaning |
|-------|---------|---------|
| `binaries` | required | comma separated binary names |
| `kind` | `cli` | `cli`, `daemon`, `app`: selects default formats (below) |
| `app-id` | | reverse-DNS id; needed for AppImage, Flatpak, macOS bundle |
| `app-name`, `description`, `name` | Cargo values | display name, one-liner, artefact base name |
| `cargo-package` | | `-p` for workspaces; also selects the package for deb/rpm/msi |
| `working-directory`, `features` | `.`, | Cargo workspace dir, `--features` for release builds |
| `linux-targets` | `x86_64,aarch64` | empty disables Linux |
| `windows-targets` | `x86_64` | `aarch64` adds a zip only; empty disables Windows |
| `macos` | `true` | universal build |
| `linux-formats`, `windows-formats`, `macos-formats` | by kind | override, see [formats.md](formats.md) |
| `apt-packages` | | build dependencies on Linux |
| `flatpak`, `flatpak-manifest` | off | Flatpak bundle; the app repo owns the manifest |
| `container`, `containerfile`, `container-image`, `container-push` | off | OCI image from the Linux binaries; push only when explicitly set |
| `android`, `ios` | off | TODO placeholders (first Blinc app); see [fdroid.md](fdroid.md) |
| `attest` | `true` | SLSA provenance (private repos need GitHub Enterprise; set false there) |
| `tag` | pushed tag | release a specific tag |

Packaging metadata stays in the app repo: `[package.metadata.deb]` and `[package.metadata.generate-rpm]` in `Cargo.toml`
(without them sensible defaults are generated), optional `wix/main.wxs`, `packaging/<app-id>.desktop`,
`packaging/<app-id>.png|svg` (AppImage), `packaging/icon.icns` (macOS app).

### Pipeline

1. `plan`: validates the tag (SemVer) and Cargo version, builds one JSON plan.
2. `linux`, `windows`, `macos`, `flatpak`, `container`, `mobile`: one job (matrix) per platform, artefacts uploaded as `dist-*`.
3. `finalize` (the only job that sees the secrets): SBOM, deb/rpm signatures, `FreeSynergy-release-key.pgp`,
   `SHA256SUMS.txt`, detached `.asc` for every file, SLSA provenance, draft release.

### Versioning the workflows

Inside fs-release, nested workflows and composite actions are referenced as `FreeSynergy/fs-release/...@main`
(GitHub cannot say "same ref as my caller" across repositories). Before tagging a release of fs-release run
`scripts/pin-refs.sh v1.0.0`, commit, tag; callers then use `@v1`. Callers may also use `@main` while developing.

### Local checks

`actionlint` and `shellcheck -x -S warning scripts/lib.sh actions/*/*.sh` (the `lint` workflow runs both).
