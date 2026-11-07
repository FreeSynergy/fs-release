# Design: packaging scripts

## Pattern map

| Where | Pattern | Why |
|-------|---------|-----|
| `actions/package-*/package.sh` | Template method | build once, then one function per requested format |
| `actions/plan/plan.py` | Facade | caller inputs in, one validated JSON plan out |
| `scripts/lib.sh` `package_groups` | Value object | a group (cargo package + its binaries) is a plain `pkg<TAB>bin,bin` line, no behaviour |
| `scripts/linux-pkg.sh` `deb_package`, `rpm_package` | Strategy | one function per native format, each builds ONE cargo package; the Template method calls it per group |
| `tests/packaging/` | none | test support |

## Rules

- A binary belongs to the cargo package that provides it (the plan's `cargo-package` wins when it provides it too).
- One deb and one rpm per package; tar.gz, AppImage, Arch, zip, macOS stay one artefact with all binaries.
- Single-package repos behave as before; `tests/golden/capnpc-*` pin plan JSON, commands and file names.
- cargo-generate-rpm `-p` is a package *directory* relative to the workspace root, cargo-deb `-p` is a package *name*.

## State

- Done: per-package build, deb, rpm; tests with stubs and a real `cargo generate-rpm` build.
- Unsupported / not done: msi (`cargo wix`) still builds one package (`cargo-package`), see below; `features` is global;
  `cargo-deb` is not installed locally, so the real deb build is tested only in CI-like environments that have it.
- msi, AppImage, Flatpak, Arch: unchanged. AppImage/Arch/zip/dmg bundle all binaries into one artefact on purpose; Flatpak builds from the app repo's manifest.
