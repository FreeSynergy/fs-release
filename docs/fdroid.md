# F-Droid: what an app repo must provide

Android apps are distributed through **F-Droid** (free, no Google account). F-Droid does not take our binaries:
it builds from source on its own servers and signs the APK with its own key (or, with reproducible builds, verifies that
our build matches and publishes our signature). The `android` job of `release.yml` is a TODO placeholder; it arrives with the
first Blinc app, which is also the test of Blinc's mobile support (problems go back to the Blinc author as PRs).

An app repository must provide:

1. **Source only, fully free**: every dependency FLOSS (ADR 0015/0016), no Google Play Services, no proprietary SDK, no
   tracking. F-Droid scans for anti-features (`NonFreeNet`, `Tracking`, ...).
2. **A reproducible build**: pinned Rust toolchain (`rust-toolchain.toml`) and NDK version, `Cargo.lock` committed,
   `--locked` builds, no network access during the build apart from fetching locked dependencies, no embedded timestamps,
   build from a git tag `vX.Y.Z` on a public repo.
3. **Version data**: `versionName` = tag without the `v`, a strictly increasing integer `versionCode` (derive it from SemVer,
   e.g. `major*1_000_000 + minor*1_000 + patch`), in the Gradle or manifest metadata.
4. **Fastlane-style metadata** in the repo (F-Droid reads it):
   `fastlane/metadata/android/en-US/{title.txt,short_description.txt,full_description.txt}`, `images/icon.png`,
   `images/featureGraphic.png`, `phoneScreenshots/*.png`, `changelogs/<versionCode>.txt`.
5. **A build recipe for fdroiddata**: a merge request to `gitlab.com/fdroid/fdroiddata` with `metadata/<app-id>.yml`
   (`Categories`, `License: MIT OR Apache-2.0` as SPDX, `SourceCode`, `IssueTracker`, `Builds:` with `commit`, `subdir`,
   `sudo` for the NDK/Rust install, `output: .../app-release-unsigned.apk`, `ndk`, `prebuild`/`build` commands, and
   `AutoUpdateMode: Version`, `UpdateCheckMode: Tags`).
6. **Optional but wanted, reproducible verification**: publish an unsigned APK from our release workflow; F-Droid compares it
   to its own build (`Binaries:` + `AllowedAPKSigningKeys` in the recipe). The `android` job will produce this unsigned APK,
   `cargo-ndk` (MIT OR Apache-2.0) or Gradle plus the Rust Android plugin.
7. **Application id**: reverse-DNS, stable forever (`net.freesynergy.<App>`), identical to `app-id` of the workflow.

iOS: build only (`aarch64-apple-ios`, simulator target), no App Store, no signing, because there is no Apple account.
