# Design: packaging scripts

## Pattern map

| Where | Pattern | Why |
|-------|---------|-----|
| `actions/package-*/package.sh` | Template method | build once, then one function per requested format |
| `actions/plan/plan.py` | Facade | caller inputs in, one validated JSON plan out |
| `scripts/lib.sh` `package_groups` | Value object | a group (cargo package + its binaries) is a plain `pkg<TAB>bin,bin` line, no behaviour |
| `scripts/linux-pkg.sh` `deb_package`, `rpm_package` | Strategy | one function per native format, each builds ONE cargo package; the Template method calls it per group |
| `actions/smoke/smoke.sh` | Strategy | one function per format (deb, zip, msi, exe): install like a person, start every binary, remove again |
| `tests/packaging/` | none | test support |

## Rules

- A binary of a workspace package under `crates/` that is not in `binaries` fails the plan (it would otherwise never be packaged). Opt out per binary or with `*` via the `ignore-binaries` input; there is no implicit default packaging.
- A binary belongs to the cargo package that provides it (the plan's `cargo-package` wins when it provides it too).
- One deb, rpm and msi per package (a single-package repo keeps `<name>-<version>-windows-<arch>.msi`, several get `<package>-<version>-windows-<arch>.msi`; an existing `wix/main.wxs` of a package is kept); tar.gz, AppImage, Arch, zip, macOS stay one artefact with all binaries.
- Single-package repos behave as before; `tests/golden/capnpc-*` pin plan JSON, commands and file names.
- cargo-generate-rpm `-p` is a package *directory* relative to the workspace root, cargo-deb `-p` is a package *name*.

## File names

- Every released file name (`dist/`, `SHA256SUMS.txt`) uses only `[A-Za-z0-9._-]`. GitHub rewrites other characters on upload (rpm `~` became `.`), which made `SHA256SUMS.txt` list names that did not exist.
- Rule: each other character becomes `-` (rpm/deb `0.6.1~rc.2` -> file `...-0.6.1-rc.2-1.x86_64.rpm`). The version *inside* the package stays `0.6.1~rc.2` (needed for correct rpm/deb ordering); only the file name changes.
- Implemented by `dist_normalise` / `dist_check` in `scripts/lib.sh`: `package-linux` normalises, the `checksums` action normalises again (SBOM etc.), checks, then hashes. Two files that would collide fail the run.

## Features

- The plan's `features` go to every `cargo build` (one per package) and to `cargo deb`, filtered to the features each package declares (`pkg_features`), because cargo rejects `-p PKG --features X` when PKG lacks X. `dep/feature` entries pass through.
- A feature declared by no built package fails before the first build. `cargo generate-rpm`, `cargo wix --no-build`, tar/zip/dmg/AppImage/Arch reuse the already built binaries and need no features.

## State

- Done: per-package build, deb, rpm; tests with stubs and a real `cargo generate-rpm` build.
- Unsupported / not done: `cargo-deb` is not installed locally, so the real deb build is tested only in CI-like environments that have it.
- Smoke test (Thread D, 2026-11-10): `actions/smoke`, called from `_release-linux.yml` (job `smoke-deb`: `debian:13` and `ubuntu:24.04`, both architectures, `docker run` with the artefacts mounted) and `_release-windows.yml` (job `smoke`: zip, msi via `msiexec /qn`, `-setup.exe /S`, uninstall checked). Every binary is started with `smoke-args` (default `--version`) and must exit 0; a failure stops `finalize`, so no draft is created. Switch off with `smoke: false`. Tested here: the deb step in real `debian:13` and `ubuntu:24.04` containers (podman) with a hand-made deb, and the call/failure paths with stubs. Run on real GitHub runners from a throwaway branch (Debian 13 and Ubuntu 24.04 on x86_64 and aarch64, Windows x86_64 zip, msi and setup.exe with uninstall; fixture `tests/fixtures/hello`, kind `app`). **Not yet run:** kind `daemon` (msi now default there), several packages in one repo, Windows arm64 (zip only), the real `finalize` gating.
- AppImage, Flatpak, Arch: unchanged. AppImage/Arch/zip/dmg bundle all binaries into one artefact on purpose; Flatpak builds from the app repo's manifest.

* **One measure (2026-11-08, Strategy; one definition 2026-11-10).** `scripts/gates.sh <root>` is the ONLY definition of the four checks: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic`, `cargo test --workspace --all-features`, `check_rust_rules.py`. `check.yml` does not repeat them; it calls `actions/gates`, which runs `gates.sh` from the same ref. `tests/packaging/gates.sh` fails if check.yml grows its own cargo commands. The local threads use this file (the copy in `.wt-doc-strategy/scripts` is older and not authoritative).
* **Pinned toolchain (2026-11-10).** `RUST_VERSION` (exact `x.y.z`) is the one Rust version for the gates: `check.yml` passes `toolchain: pinned` to `actions/setup-rust`, `gates.sh` exports it as `RUSTUP_TOOLCHAIN`. Programs take it over without a file of their own. Reason: with unpinned stable, every new clippy lint turned all repos red at once (~228 red runs). Release builds keep newest stable (ADR 0026) and are unaffected. Update path: a deliberate bump PR in fs-release that changes only `RUST_VERSION`; before merging, run `scripts/gates.sh` on every program repo (fresh clones of main) with the new version; repos that turn red are fixed first, then the bump merges. Because programs use `check.yml@main`, merging the bump takes effect everywhere at once, so it never merges red.
