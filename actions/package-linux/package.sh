#!/usr/bin/env bash
# Pattern: Template method - build once, then one small function per requested format.
set -euo pipefail
# shellcheck source=../../scripts/lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/../../scripts/lib.sh"
# shellcheck source=../../scripts/linux-pkg.sh
source "$(dirname "${BASH_SOURCE[0]}")/../../scripts/linux-pkg.sh"

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
groups=(); group_list   # one entry per cargo package: "<package>TAB<bin,bin>"

# for_each_group FUNC -> FUNC PKG BINS, once per cargo package (deb and rpm are built per package).
for_each_group() { local g; for g in "${groups[@]}"; do "$1" "${g%%$'\t'*}" "${g#*$'\t'}"; done; }

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

make_deb() { for_each_group deb_one; }
deb_one() { deb_package "$1" "$TRIPLE" "$dist"; }

make_rpm() { for_each_group rpm_one; }
rpm_one() { rpm_package "$1" "$2" "$TRIPLE" "$dist"; }

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
dist_normalise "$dist"
ls -l "$dist"
