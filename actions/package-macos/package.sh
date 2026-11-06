#!/usr/bin/env bash
# Pattern: Template method. Build both architectures, lipo, ad-hoc sign, then tar.gz (CLI) or dmg (app).
# Ad-hoc signing (codesign -s -) needs no Apple account; arm64 macOS refuses unsigned code after lipo.
set -euo pipefail
# shellcheck source=../../scripts/lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/../../scripts/lib.sh"

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
name=$(plan_get name) version=$(plan_get version) core=$(plan_get version_core)
formats=$(plan_get macos_formats)
dist=$(dist_dir)
in_workdir
base="$name-$version-macos-universal"
read -ra bins <<<"$(plan_get binaries | tr ',' ' ')"
triples=(x86_64-apple-darwin aarch64-apple-darwin)

for t in "${triples[@]}"; do group "build $t"; cargo_build "$t"; endgroup; done

stage="$RUNNER_TEMP/stage/$base"
rm -rf "$stage"; mkdir -p "$stage"
for b in "${bins[@]}"; do
  lipo -create -output "$stage/$b" "$(bin_path "${triples[0]}" "$b")" "$(bin_path "${triples[1]}" "$b")"
  strip -x "$stage/$b"
  codesign --force --sign - "$stage/$b"
  lipo -info "$stage/$b"
done
copy_docs "$stage"
cp "$here/open-anyway.txt" "$stage/OPEN-ANYWAY.txt"

make_tarball() {
  tar -C "$RUNNER_TEMP/stage" -czf "$dist/$base.tar.gz" "$base"
}

make_dmg() {
  local app_name app_id main app contents dmg_src
  app_name=$(plan_get app_name) app_id=$(plan_get app_id) main=${bins[0]}
  dmg_src="$RUNNER_TEMP/dmg"; rm -rf "$dmg_src"; mkdir -p "$dmg_src"
  app="$dmg_src/$app_name.app"; contents="$app/Contents"
  mkdir -p "$contents/MacOS" "$contents/Resources"
  for b in "${bins[@]}"; do cp "$stage/$b" "$contents/MacOS/"; done
  [[ -f packaging/icon.icns ]] && cp packaging/icon.icns "$contents/Resources/icon.icns"
  cat > "$contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>$app_name</string>
  <key>CFBundleIdentifier</key><string>${app_id:-net.freesynergy.$name}</string>
  <key>CFBundleExecutable</key><string>$main</string>
  <key>CFBundleShortVersionString</key><string>$core</string>
  <key>CFBundleVersion</key><string>$core</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
  codesign --force --deep --sign - "$app"
  ln -s /Applications "$dmg_src/Applications"
  cp "$here/open-anyway.txt" "$dmg_src/OPEN-ANYWAY.txt"
  hdiutil create -volname "$app_name" -srcfolder "$dmg_src" -ov -format UDZO "$dist/$base.dmg"
}

for f in tar.gz dmg; do
  has_format "$formats" "$f" || continue
  group "package $f"
  case "$f" in tar.gz) make_tarball ;; dmg) make_dmg ;; esac
  endgroup
done
ls -l "$dist"
