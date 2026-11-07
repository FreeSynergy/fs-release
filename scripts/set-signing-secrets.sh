#!/usr/bin/env bash
# Set the release-signing secrets in FreeSynergy repositories, in one command.
#
#   scripts/set-signing-secrets.sh fs-release          # one repository
#   scripts/set-signing-secrets.sh fs-tasks fs-people  # several
#   scripts/set-signing-secrets.sh all                 # every non-archived fs-* repository
#
# What it does:
#   1. finds the newest valid signing subkey of the release key (creates one, ed25519, 1 year, if there is none),
#   2. exports ONLY that secret subkey (the primary key never leaves the keyring),
#   3. sets GPG_PRIVATE_KEY and GPG_PASSPHRASE in each repository (FreeSynergy is a user account: no org secrets),
#   4. writes the public key (primary + subkeys) and the subkey fingerprint to $OUT_DIR, owned by $OUT_OWNER,
#   5. uploads the public key to keys.openpgp.org (skip with NO_UPLOAD=1).
#
# Settings (environment):
#   PASSFILE   file with the passphrase, mode 600   (default: ~/.config/freesynergy/release-passphrase)
#   GNUPGHOME  keyring to use                        (default: gpg's default)
#   OUT_DIR    where the public key goes             (default: /srv/FreeSynergy/.scratch/keys)
#   OUT_OWNER  owner of the written files            (default: kal:kal)
# Needs: gpg, gh (logged in as FreeSynergy), curl.
set -euo pipefail

PRIMARY=16A4A3EE563AEF51FCF0C55DED7B57C4A1904342
PASSFILE=${PASSFILE:-$HOME/.config/freesynergy/release-passphrase}
OUT_DIR=${OUT_DIR:-/srv/FreeSynergy/.scratch/keys}
OUT_OWNER=${OUT_OWNER:-kal:kal}
OWNER_ACCOUNT=FreeSynergy

die() { echo "error: $*" >&2; exit 1; }

[ $# -ge 1 ] || die "name at least one repository, or 'all'"
[ -f "$PASSFILE" ] || die "passphrase file $PASSFILE not found"
[ "$(stat -c %a "$PASSFILE")" = 600 ] || die "$PASSFILE must have mode 600 (chmod 600 $PASSFILE)"
for tool in gpg gh curl; do command -v "$tool" >/dev/null || die "$tool is needed"; done

who=$(gh api user -q .login) || die "gh is not logged in (gh auth login)"
[ "$who" = "$OWNER_ACCOUNT" ] || die "gh is logged in as '$who', not '$OWNER_ACCOUNT' (gh auth switch -u $OWNER_ACCOUNT)"

gpg --list-secret-keys "$PRIMARY" >/dev/null 2>&1 || die "secret key $PRIMARY is not in this keyring"

GPGP=(gpg --batch --pinentry-mode loopback --passphrase-file "$PASSFILE")

newest_sign_subkey() {
  # ssb/sub lines: field 2 validity, 6 created, 12 capabilities; fpr line follows.
  gpg --with-colons --with-subkey-fingerprints --list-secret-keys "$PRIMARY" | awk -F: -v now="$(date +%s)" '
    $1=="ssb" { ok = ($2 !~ /[rei]/) && ($12 ~ /s/) && ($7=="" || $7>now); created=$6; want=1; next }
    $1=="fpr" && want { if (ok && created>best) { best=created; fpr=$10 } want=0 }
    END { print fpr }'
}

SUB=$(newest_sign_subkey)
if [ -z "$SUB" ]; then
  echo "no valid signing subkey: creating one (ed25519, 1 year)"
  "${GPGP[@]}" --quick-add-key "$PRIMARY" ed25519 sign 1y
  SUB=$(newest_sign_subkey)
  [ -n "$SUB" ] || die "could not create a signing subkey"
fi
echo "signing subkey: $SUB"

TMP=$(mktemp -d /dev/shm/fs-sign.XXXXXX)
chmod 700 "$TMP"
trap 'shred -u "$TMP"/* 2>/dev/null || true; rmdir "$TMP" 2>/dev/null || true' EXIT
"${GPGP[@]}" --armor --export-secret-subkeys "${SUB}!" > "$TMP/sub.asc"
grep -q "BEGIN PGP PRIVATE KEY BLOCK" "$TMP/sub.asc" || die "export of the subkey failed"

if [ "$1" = all ]; then
  mapfile -t REPOS < <(gh repo list "$OWNER_ACCOUNT" --limit 200 --json name,isArchived \
    -q '.[] | select(.isArchived|not) | .name' | grep '^fs-' | grep -v -- '-archive$')
else
  REPOS=("$@")
fi

for r in "${REPOS[@]}"; do
  gh secret set GPG_PRIVATE_KEY -R "$OWNER_ACCOUNT/$r" < "$TMP/sub.asc"
  gh secret set GPG_PASSPHRASE  -R "$OWNER_ACCOUNT/$r" < "$PASSFILE"
  echo "secrets set: $r"
done

mkdir -p "$OUT_DIR"
gpg --armor --export "$PRIMARY" > "$OUT_DIR/release-signing-key.asc"
echo "$SUB" > "$OUT_DIR/signing-subkey-fingerprint.txt"
chown "$OUT_OWNER" "$OUT_DIR" "$OUT_DIR/release-signing-key.asc" "$OUT_DIR/signing-subkey-fingerprint.txt" 2>/dev/null \
  || echo "note: could not chown to $OUT_OWNER (run as root or as that user)"
echo "public key written: $OUT_DIR/release-signing-key.asc"

if [ "${NO_UPLOAD:-0}" != 1 ]; then
  gpg --export "$PRIMARY" | curl -fsS -T - https://keys.openpgp.org >/dev/null && echo "uploaded to keys.openpgp.org"
fi
echo "done"
