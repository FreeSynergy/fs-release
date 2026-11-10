# Design: packaging scripts

## Pattern map

| Where | Pattern | Why |
|-------|---------|-----|
| `actions/package-*/package.sh` | Template method | build once, then one function per requested format |
| `actions/plan/plan.py` | Facade | caller inputs in, one validated JSON plan out |
| `scripts/lib.sh` `package_groups` | Value object | a group (cargo package + its binaries) is a plain `pkg<TAB>bin,bin` line, no behaviour |
| `scripts/linux-pkg.sh` `deb_package`, `rpm_package` | Strategy | one function per native format, each builds ONE cargo package; the Template method calls it per group |
| `actions/smoke/smoke.sh` | Strategy | one function per format (deb, zip, msi, exe): install like a person, start every binary, remove again |
| `scripts/bump-components.sh` | Template method | per program repo the same steps: fresh worktree, set the tag (`set_components_tag.py`), `cargo update`, gates, push the branch (does not convert programs) |
| `scripts/check_rust_rules.py` `check_blinc` | none | rule, per repo on via `.fs-rules.toml` `blinc = "components-only"`: no `blinc*` dependency, `[patch]`/`[replace]`, one fs-components tag |
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

- `scripts/bump-components.sh [--dry-run] <tag> [repo ...]` lives here and not in fs-components: it is release machinery over many repos (it uses `gates.sh`, the same measure as CI); fs-components is a library and must not know its dependants. First it checks that the tag exists in the fs-components repo (`git ls-remote --tags`). Per repo it creates a FRESH worktree `.wt-bump-<repo>` from `origin/main` (an existing one is an error, never reused: it may hold old state), lets `scripts/set_components_tag.py` set `tag = "<tag>"` on EVERY dependency whose `git` is the fs-components repo (any name, `package =`, inline or table form, workspace/dev/build/target tables, Cargo.toml at any depth outside `target/` and `.wt-*`; rev/branch removed) and verifies the result with a TOML parser (a form it cannot edit fails, nothing is written), runs `cargo update -p <url#name@version>` for every package of that git source (the spec rules out a same-named package from another source), `gates.sh`, and pushes `HEAD:refs/heads/bump-components-<tag>` only when green. Already on the tag = ok and nothing pushed; no fs-components dependency found = failure. Never a tag, never a merge; `--dry-run` stops before the push. Each repo runs in its own shell, so one red repo does not stop the others (exit 1 at the end).
- The script does NOT convert programs: it removes no `blinc*` line and no `[patch]` and rewrites no `use`. The first move of a repo to "Blinc only through fs-components" is manual work per repo; it ends by adding the switch below. Only then does the script keep the repo current.
- Rule in `check_rust_rules.py` (`check_blinc`), switched per repo: the programs call `fs-release/.github/workflows/check.yml@main`, so a rule that is always on would turn every program's CI red at once (they still carry `blinc_*` and `[patch]`). The rule runs only when the repo root has `.fs-rules.toml` with `blinc = "components-only"` (default `off`; any other value is an error). Migrating a repo by hand includes adding that file. When on, a repo fails on: a `blinc*` dependency (also renamed through `package =`, by git url, in `target.*`, dev and build tables), a `[patch]` or `[replace]` entry naming blinc in any Cargo.toml or `.cargo/config.toml`, and two different fs-components tags/revs in one program (two Blinc). Exempt only the real fs-components repo: `crates/fs-components/Cargo.toml` must be a package named `fs-components` (and name that repository if it names one); an empty file there is no exemption. Cargo.toml in `target/`, `.git/` and nested `.wt-*` worktrees are not looked at. Cargo applies `[patch]` only at the top of a build, so such a line in a program is either redundant or a second, diverging pin.
- Tests: `tests/packaging/rules.sh`, `tests/packaging/bump.sh` (a fake origin and a file:// fs-components repo with tags, stub gates, one run with the real `cargo update -p`).
