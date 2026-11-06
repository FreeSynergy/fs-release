#!/usr/bin/env bash
# Pattern: Adapter - a PKGBUILD that installs the prebuilt binaries from our own tarball.
# makepkg refuses to run as root, so the build happens as the throw-away user "builder".
set -euo pipefail
# shellcheck source=../../scripts/lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/../../scripts/lib.sh"
dist=$1

name=$(plan_get name) version=$(plan_get version)
base="$name-$version-linux-x86_64"
[[ -f "$dist/$base.tar.gz" ]] || die "$dist/$base.tar.gz missing (the arch format needs the x86_64 tar.gz)"

id builder >/dev/null 2>&1 || useradd -m builder
work=/home/builder/pkg
rm -rf "$work"; mkdir -p "$work"
cp "$dist/$base.tar.gz" "$work/"
sum=$(sha256sum "$work/$base.tar.gz" | cut -d' ' -f1)

{
  echo "pkgname='$name'"
  echo "pkgver='${version//-/_}'   # makepkg forbids '-' in pkgver"
  echo "pkgrel=1"
  echo "pkgdesc='$(plan_get description | tr -d "'")'"
  echo "arch=('x86_64')"
  echo "url='$(plan_get homepage)'"
  echo "license=('$(plan_get license | tr -d "'")')"
  echo "options=('!strip')"
  echo "source=('$base.tar.gz')"
  echo "sha256sums=('$sum')"
  echo "package() {"
  for b in $(plan_get binaries | tr ',' ' '); do
    echo "  install -Dm755 '$base/$b' \"\$pkgdir/usr/bin/$b\""
  done
  echo "  for l in $base/LICENSE*; do [ -f \"\$l\" ] && install -Dm644 \"\$l\" \"\$pkgdir/usr/share/licenses/\$pkgname/\${l##*/}\"; done; true"
  echo "}"
} > "$work/PKGBUILD"
chown -R builder: "$work"

su builder -c "cd '$work' && makepkg --nodeps --noconfirm --cleanbuild"
cp "$work"/*.pkg.tar.zst "$dist/"
ls -l "$dist"
