# Design: packaging scripts

## Pattern map

| Where | Pattern | Why |
|-------|---------|-----|
| `actions/package-*/package.sh` | Template method | build once, then one function per requested format |
| `actions/plan/plan.py` | Facade | caller inputs in, one validated JSON plan out |
| `scripts/lib.sh` `package_groups` | Value object | a group (cargo package + its binaries) is a plain `pkg<TAB>bin,bin` line, no behaviour |
| `scripts/linux-pkg.sh` `deb_package`, `rpm_package` | Strategy | one function per native format, each builds ONE cargo package; the Template method calls it per group |
| `actions/smoke/smoke.sh` | Strategy | one function per format (deb, zip, msi, exe): install like a person, start every binary, remove again |
| `scripts/bump-components.sh` | Template method | per program repo the same steps: worktree, set the tag, `cargo update`, gates, push the branch |
| `scripts/check_rust_rules.py` `check_blinc` | none | rule: no `blinc*` dependency and no blinc `[patch]` outside fs-components |
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

* **One measure (2026-11-08, Strategy).** `scripts/gates.sh <root>` and `.github/workflows/check.yml` run exactly the same four checks: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic`, `cargo test --workspace --all-features`, `check_rust_rules.py`. Change both together; the local copy used by the threads is `/srv/FreeSynergy/.scratch/t5-gates/gates.sh` (a copy of this file).

## Blinc in one place

- `scripts/bump-components.sh [--dry-run] <tag> [repo ...]` lives here and not in fs-components: it is release machinery over many repos (it uses `gates.sh`, the same measure as CI, which also lives here); fs-components is a library and must not know its dependants. It works in `.wt-bump-<repo>` worktrees from `origin/main`, sets the tag on every `fs-components*` git dependency, runs `cargo update -p` for those packages, `gates.sh`, and pushes the branch `bump-components-<tag>` only when green. Never a tag, never a merge; `--dry-run` stops before the push. Each repo runs in its own shell, so one red repo does not stop the others (exit 1 at the end).
- Rule in `check_rust_rules.py`: every repo except the one with a crate `crates/fs-components` fails on a `blinc*` dependency (also renamed through `package =`, also `target.*`, dev and build tables) or a `[patch]` entry that names blinc. Cargo applies `[patch]` only at the top of a build, so such a line in a program is either redundant or a second, diverging pin.
- Tests: `tests/packaging/rules.sh`, `tests/packaging/bump.sh` (a fake origin, stub gates).
