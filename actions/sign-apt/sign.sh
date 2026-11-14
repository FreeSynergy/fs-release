#!/usr/bin/env bash
# Pattern: Strategy-by-presence (like sign-files). Signs dists/<suite>/Release of EVERY suite: InRelease (clearsign) + Release.gpg (detached, armoured).
# Same hygiene as sign-files/sign.sh: private temp GNUPGHOME (0700) and passphrase file (0600), removed by a trap on EVERY exit.
# Unlike sign-files it FAILS when the key is missing: an APT repository must never be published unsigned.
# The signature is made by exactly ONE signing subkey (SIGN_SUBKEY, forced with "!"): the secret holds only the subkey (gpg --export-secret-subkeys),
# the primary key stays offline; a secret without that subkey is an error.
# Usage: sign.sh REPO_DIR       (env GPG_PRIVATE_KEY, GPG_PASSPHRASE, SIGN_SUBKEY). Test: tests/packaging/sign-apt.sh.
set -euo pipefail
dir=${1:?usage: sign.sh REPO_DIR}
if [[ -z "${GPG_PRIVATE_KEY:-}" ]]; then
  echo "::error::GPG_PRIVATE_KEY is not set - an APT repository is never published unsigned"
  exit 1
fi
shopt -s nullglob
releases=("$dir"/dists/*/Release)
((${#releases[@]})) || { echo "::error::no dists/<suite>/Release below $dir"; exit 1; }

umask 077
tmp_base="${RUNNER_TEMP:-/tmp}"
GNUPGHOME="$(mktemp -d "$tmp_base/fs-release-gnupg.XXXXXX")"
export GNUPGHOME
pass_file="$(mktemp "$tmp_base/fs-release-pass.XXXXXX")"
cleanup() {
  gpgconf --kill gpg-agent >/dev/null 2>&1 || true
  [[ -n "${pass_file:-}" && -f "$pass_file" ]] && { shred -u "$pass_file" 2>/dev/null || rm -f "$pass_file"; }
  [[ -n "${GNUPGHOME:-}" && -d "$GNUPGHOME" ]] && rm -rf "$GNUPGHOME"
  return 0
}
trap cleanup EXIT
chmod 700 "$GNUPGHOME"; chmod 600 "$pass_file"
printf '%s' "${GPG_PASSPHRASE:-}" > "$pass_file"
printf '%s' "$GPG_PRIVATE_KEY" | gpg --batch --import
sub=${SIGN_SUBKEY:-}; sub=${sub^^}; sub=${sub// /}
[[ $sub =~ ^[0-9A-F]{40}$ ]] || { echo "::error::SIGN_SUBKEY must be the 40 hex digit fingerprint of the signing subkey"; exit 1; }
gpg --batch --list-secret-keys --with-colons | awk -F: '/^ssb:/ {s=1; next} s && /^fpr:/ {print toupper($10); s=0}' | grep -qx "$sub" \
  || { echo "::error::signing subkey $sub not found in GPG_PRIVATE_KEY"; exit 1; }
# "!" forces exactly this subkey (gpg would otherwise pick the newest valid signing subkey)
gpg_opts=(--batch --yes --pinentry-mode loopback --passphrase-file "$pass_file" --local-user "$sub!" --digest-algo SHA512)
echo "Signing subkey fingerprint: $sub"
for rel in "${releases[@]}"; do
  d=$(dirname "$rel")
  rm -f "$d/InRelease" "$d/Release.gpg"
  gpg "${gpg_opts[@]}" --clearsign --output "$d/InRelease" "$rel"
  gpg "${gpg_opts[@]}" --armor --detach-sign --output "$d/Release.gpg" "$rel"
  echo "signed $d"
done
