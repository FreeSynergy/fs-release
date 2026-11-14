#!/usr/bin/env bash
# Pattern: none (test). actions/sign-apt/sign.sh signs every suite (InRelease + Release.gpg), the signatures verify with gpgv,
# a manipulated Release fails, and nothing is left behind (GNUPGHOME, passphrase file, agent); no key / bad key / no Release = failure.
TS="$T/signapt"; mkdir -p "$TS/rt" "$TS/gen"
if ! command -v gpg >/dev/null || ! command -v gpgv >/dev/null; then pass "sign-apt: gpg/gpgv missing, skipped"; else
  chmod 700 "$TS/gen"
  # throwaway key like the project key: primary [C] offline + TWO signing subkeys; the secret holds ONLY the subkeys
  GG() { GNUPGHOME="$TS/gen" gpg --batch --quiet --pinentry-mode loopback --passphrase secret-pass "$@" 2>/dev/null; }
  GG --quick-generate-key "t <t@example.org>" ed25519 cert 0
  PRIM=$(GNUPGHOME="$TS/gen" gpg --batch --list-keys --with-colons 2>/dev/null | awk -F: '/^fpr:/ {print $10; exit}')
  GG --quick-add-key "$PRIM" ed25519 sign 0
  GG --quick-add-key "$PRIM" ed25519 sign 0
  mapfile -t SUBS < <(GNUPGHOME="$TS/gen" gpg --batch --list-keys --with-colons 2>/dev/null | awk -F: '/^sub:/ {s=1; next} s && /^fpr:/ {print $10; s=0}')
  SUB=${SUBS[0]}; OTHER=${SUBS[1]}
  KEY=$(GG --armor --export-secret-subkeys "$SUB!")
  GNUPGHOME="$TS/gen" gpg --batch --export > "$TS/pub.pgp" 2>/dev/null
  GNUPGHOME="$TS/gen" gpgconf --kill gpg-agent 2>/dev/null || true
  [[ -n $SUB && -n $OTHER && $SUB != "$OTHER" && -n $KEY ]] && pass "sign-apt: test key with 2 signing subkeys, subkey-only secret exported" || fail "sign-apt: test key setup failed"
  for s in stable testing; do mkdir -p "$TS/repo/dists/$s"; printf 'Suite: %s\n' "$s" > "$TS/repo/dists/$s/Release"; done
  S() { RUNNER_TEMP="$TS/rt" SIGN_SUBKEY="${SUBKEY-$SUB}" GPG_PRIVATE_KEY="$KEY" GPG_PASSPHRASE=secret-pass "$ROOT/actions/sign-apt/sign.sh" "$@" >"$TS/out" 2>&1; }
  S "$TS/repo" && pass "sign-apt: run succeeded" || fail "sign-apt: run failed: $(cat "$TS/out")"
  for s in stable testing; do
    [[ -s "$TS/repo/dists/$s/InRelease" && -s "$TS/repo/dists/$s/Release.gpg" ]] && pass "sign-apt: $s has InRelease and Release.gpg" || fail "sign-apt: $s signature files missing"
    gpgv --homedir "$TS/gen" --keyring "$TS/pub.pgp" "$TS/repo/dists/$s/Release.gpg" "$TS/repo/dists/$s/Release" >/dev/null 2>&1 && pass "sign-apt: $s Release.gpg verifies" || fail "sign-apt: $s Release.gpg does not verify"
    gpgv --homedir "$TS/gen" --keyring "$TS/pub.pgp" "$TS/repo/dists/$s/InRelease" >/dev/null 2>&1 && pass "sign-apt: $s InRelease verifies" || fail "sign-apt: $s InRelease does not verify"
  done
  st=$(gpgv --homedir "$TS/gen" --keyring "$TS/pub.pgp" --status-fd=1 "$TS/repo/dists/stable/Release.gpg" "$TS/repo/dists/stable/Release" 2>/dev/null || true)
  [[ $(awk '$2=="VALIDSIG" {print toupper($3)}' <<< "$st") == "${SUB^^}" && $(awk '$2=="VALIDSIG" {print toupper($12)}' <<< "$st") == "${PRIM^^}" ]] && pass "sign-apt: signed by exactly the chosen subkey of the primary key" || fail "sign-apt: wrong signing key: $st"
  SUBKEY="$OTHER" S "$TS/repo" && fail "sign-apt: a subkey that is not in the secret must fail" || pass "sign-apt: subkey not in the secret fails"
  SUBKEY="" S "$TS/repo" && fail "sign-apt: missing SIGN_SUBKEY must fail" || pass "sign-apt: missing SIGN_SUBKEY fails"
  echo tampered >> "$TS/repo/dists/stable/Release"
  gpgv --homedir "$TS/gen" --keyring "$TS/pub.pgp" "$TS/repo/dists/stable/Release.gpg" "$TS/repo/dists/stable/Release" >/dev/null 2>&1 && fail "sign-apt: manipulated Release must not verify" || pass "sign-apt: manipulated Release fails"
  grep -q -e 'secret-pass' -e 'PRIVATE KEY' "$TS/out" && fail "sign-apt: output shows key material" || pass "sign-apt: output shows no key material"
  [[ -z "$(ls -A "$TS/rt")" ]] && pass "sign-apt: nothing left in RUNNER_TEMP" || fail "sign-apt: left in RUNNER_TEMP: $(ls -A "$TS/rt")"
  RUNNER_TEMP="$TS/rt" GPG_PRIVATE_KEY="" "$ROOT/actions/sign-apt/sign.sh" "$TS/repo" >/dev/null 2>&1 && fail "sign-apt: missing key must fail" || pass "sign-apt: missing key fails (never unsigned)"
  RUNNER_TEMP="$TS/rt" GPG_PRIVATE_KEY="not a key" "$ROOT/actions/sign-apt/sign.sh" "$TS/repo" >/dev/null 2>&1 && fail "sign-apt: broken key must fail" || pass "sign-apt: broken key fails"
  mkdir -p "$TS/empty"
  S "$TS/empty" && fail "sign-apt: no Release must fail" || pass "sign-apt: no dists/<suite>/Release fails"
  [[ -z "$(ls -A "$TS/rt")" ]] && pass "sign-apt: nothing left after failures either" || fail "sign-apt: left after failures: $(ls -A "$TS/rt")"
  grep -q 'trap cleanup EXIT' "$ROOT/actions/sign-apt/sign.sh" && pass "sign-apt: EXIT trap present" || fail "sign-apt: EXIT trap missing"
fi
