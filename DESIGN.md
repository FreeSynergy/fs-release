# Design: packaging scripts

## Pattern map

| Where | Pattern | Why |
|-------|---------|-----|
| `actions/package-*/package.sh` | Template method | build once, then one function per requested format |
| `actions/plan/plan.py` | Facade | caller inputs in, one validated JSON plan out |
| `scripts/lib.sh` `package_groups` | Value object | a group (cargo package + its binaries) is a plain `pkg<TAB>bin,bin` line, no behaviour |
| `scripts/linux-pkg.sh` `deb_package`, `rpm_package` | Strategy | one function per native format, each builds ONE cargo package; the Template method calls it per group |
| `actions/smoke/smoke.sh` | Strategy | one function per format (deb, zip, msi, exe): install like a person, start every binary, remove again |
| `scripts/typing-test.sh` + `scripts/typing-test/` | Facade | one command (binary in, exit code and junit/json out) over a virtual KWin, the key injector (`inject/`, a small Wayland client: Adapter to KWin's `org_kde_kwin_fake_input`), the pictures (`session.sh`) and the checks (`analyze.py`, `fieldcheck.py`) |
| `scripts/typing-test/session.sh` | Template method | wait for the window, find the field by its caret, type one character, picture, clear; then two text parts; then the timed run. Apps may replace `focus()` with a scenario file |
| `actions/typing-test` | Facade | the same script as a composite action for the app repos (see below) |
| `tests/packaging/`, `tests/typing-test/` | none | test support |

## Typing test (`actions/typing-test`, `scripts/typing-test.sh`)

Interface of the action (inputs): `app-binary` (required), `app-args`, `click-in-window` (x,y from the window's top left corner; preferred) or `click` (on the 1280x800 screen) or `scenario` (bash fragment defining `focus()` for apps that need clicks to reach the field), `field`, `text-width`, `layout` (default `de`), `max-ms` (default 8), `enforce-ms`, `size`, `settle`, `inject`, `name`, `install`. Outputs: artefact `typing-test-<name>` with `junit.xml`, `result.json`, `shots/` and logs; the step fails if a check fails (exit 1) or the test could not run (exit 2).

What is checked: each of `a Ä ä ö ü ß @ € { } [ ] \ | ~ µ` (Shift and AltGr) arrives in the focused field and is drawn as that glyph (compared with a reference rendering of all printable ASCII and the German extras in DejaVu Sans, the typed one must be among the best three; `|` is only told apart from bars), the text is visible and the caret stands right behind it, and the median key-to-picture time (client Wayland trace) is within `max-ms`. A wrong layout turns it red. Without a GPU (GitHub runner) the time is reported, not enforced.

Isolation: own runtime directory and private D-Bus, own Wayland socket, no `DISPLAY`, toolkits forced to Wayland, own process group killed at the end; `tests/typing-test/isolation.sh` proves it. Known limits: picture-based (no accessibility read-out); window placement is by KWin 5.27 (Ubuntu) or 6.x (laptop), so clicks are relative to the window; pictures cost about one second each, a run takes minutes. The probe workflow `typing-test-probe` runs on push to the branch only; after the merge it is `workflow_dispatch` only.

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
