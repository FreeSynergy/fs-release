#!/usr/bin/env bash
# Pattern: Template method - build once, then one small function per requested format.
set -euo pipefail
# shellcheck source=../../scripts/lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/../../scripts/lib.sh"

# Pinned appimagetool (MIT). Bump together with the checksums.
APPIMAGETOOL_VERSION=1.9.1
declare -A APPIMAGETOOL_SHA256=(
  [x86_64]=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
  [aarch64]=f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158
)

name=$(plan_get name) version=$(plan_get version) formats=$(plan_get linux_formats)
dist=$(dist_dir)
in_workdir
base="$name-$version-linux-$ARCH"
read -ra bins <<<"$(plan_get binaries | tr ',' ' ')"
pkg=(); while IFS= read -r _l; do pkg+=("$_l"); done < <(cargo_pkg_args)   # bash 3.2 (macOS) has no mapfile

group "build $TRIPLE"
cargo_build "$TRIPLE"
for b in "${bins[@]}"; do strip "$(bin_path "$TRIPLE" "$b")"; done
endgroup

stage_tree() { # stage_tree DIR -> binaries + docs
  mkdir -p "$1"
  for b in "${bins[@]}"; do install -m 755 "$(bin_path "$TRIPLE" "$b")" "$1/"; done
  copy_docs "$1"
}

make_tarball() {
  local s="$RUNNER_TEMP/stage/$base"
  rm -rf "$s"; stage_tree "$s"
  tar -C "$RUNNER_TEMP/stage" --sort=name --owner=0 --group=0 --numeric-owner \
    --mtime="@${SOURCE_DATE_EPOCH:-0}" -czf "$dist/$base.tar.gz" "$base"
}

make_deb() { cargo deb --no-build --target "$TRIPLE" ${pkg[@]+"${pkg[@]}"} --output "$dist/"; }

make_rpm() {
  # cargo-generate-rpm can only overwrite an existing [package.metadata.generate-rpm] table, not create one.
  # If the package has none, append it to the manifest of this CI checkout only (never committed).
  local manifest b assets=""
  manifest=$(cargo metadata --format-version 1 --no-deps \
    | jq -r --arg b "${bins[0]}" '.packages[] | select(any(.targets[]; .name == $b and (.kind | index("bin")))) | .manifest_path' | head -n1)
  [[ -n "$manifest" ]] || die "no package provides binary ${bins[0]}"
  if ! grep -q 'package.metadata.generate-rpm' "$manifest"; then
    for b in "${bins[@]}"; do
      assets+="{ source = \"$(bin_path "$TRIPLE" "$b")\", dest = \"/usr/bin/$b\", mode = \"755\" }, "
    done
    printf '\n[package.metadata.generate-rpm]\nassets = [%s]\n' "${assets%, }" >> "$manifest"
  fi
  # RPM forbids '-' in versions; a SemVer pre-release becomes '~' (sorts before the release, e.g. 1.0~rc.1 < 1.0).
  local v; v=$(plan_get version); v=${v/-/\~}
  cargo generate-rpm --target "$TRIPLE" ${pkg[@]+"${pkg[@]}"} -s "version = \"$v\"" --output "$dist/"
}

make_appimage() {
  local app_id app_name appdir main tool
  app_id=$(plan_get app_id); app_name=$(plan_get app_name); main=${bins[0]}
  [[ -n "$app_id" ]] || die "AppImage needs the 'app-id' input"
  appdir="$RUNNER_TEMP/AppDir"; rm -rf "$appdir"; mkdir -p "$appdir/usr/bin"
  for b in "${bins[@]}"; do install -m 755 "$(bin_path "$TRIPLE" "$b")" "$appdir/usr/bin/"; done
  ln -s "usr/bin/$main" "$appdir/AppRun"
  if [[ -f "packaging/$app_id.desktop" ]]; then cp "packaging/$app_id.desktop" "$appdir/$app_id.desktop"; else
    printf '[Desktop Entry]\nType=Application\nName=%s\nComment=%s\nExec=%s\nIcon=%s\nTerminal=%s\nCategories=Utility;\n' \
      "$app_name" "$(plan_get description)" "$main" "$app_id" \
      "$([[ $(plan_get kind) == app ]] && echo false || echo true)" > "$appdir/$app_id.desktop"
  fi
  if [[ -f "packaging/$app_id.png" ]]; then cp "packaging/$app_id.png" "$appdir/$app_id.png"
  elif [[ -f "packaging/$app_id.svg" ]]; then cp "packaging/$app_id.svg" "$appdir/$app_id.svg"
  else
    warn "no packaging/$app_id.png - using a placeholder icon"
    python3 -I "$(dirname "${BASH_SOURCE[0]}")/placeholder_icon.py" "$appdir/$app_id.png"
  fi
  tool="$RUNNER_TEMP/appimagetool"
  curl -fsSL -o "$tool" \
    "https://github.com/AppImage/appimagetool/releases/download/$APPIMAGETOOL_VERSION/appimagetool-$ARCH.AppImage"
  echo "${APPIMAGETOOL_SHA256[$ARCH]}  $tool" | sha256sum -c -
  chmod +x "$tool"
  ARCH=$ARCH "$tool" --appimage-extract-and-run "$appdir" "$dist/$base.AppImage"
}

for f in tar.gz deb rpm appimage; do
  has_format "$formats" "$f" || continue
  group "package $f"
  case "$f" in
    tar.gz) make_tarball ;; deb) make_deb ;; rpm) make_rpm ;; appimage) make_appimage ;;
  esac
  endgroup
done
ls -l "$dist"
