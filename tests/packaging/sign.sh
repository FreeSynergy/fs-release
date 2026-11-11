#!/usr/bin/env bash
# Pattern: none (test). actions/sign-files/sign.sh signs and leaves NOTHING behind: no GNUPGHOME, no passphrase file, no agent.
TS="$T/sign"; mkdir -p "$TS/rt" "$TS/dist" "$TS/gen"
if ! command -v gpg >/dev/null; then pass "sign: gpg missing, skipped"; else
  export GNUPGHOME_GEN="$TS/gen"; chmod 700 "$TS/gen"
  printf 'Key-Type: EDDSA\nKey-Curve: ed25519\nName-Real: t\nName-Email: t@example.org\nExpire-Date: 0\nPassphrase: secret-pass\n%%commit\n' > "$TS/gen/spec"
  GNUPGHOME="$TS/gen" gpg --batch --quiet --generate-key "$TS/gen/spec" 2>/dev/null
  KEY=$(GNUPGHOME="$TS/gen" gpg --batch --armor --pinentry-mode loopback --passphrase secret-pass --export-secret-keys 2>/dev/null)
  GNUPGHOME="$TS/gen" gpgconf --kill gpg-agent 2>/dev/null || true
  echo data > "$TS/dist/file.txt"
  S() { RUNNER_TEMP="$TS/rt" GPG_PRIVATE_KEY="$KEY" GPG_PASSPHRASE=secret-pass "$ROOT/actions/sign-files/sign.sh" "$@" >"$TS/out" 2>&1; }
  S detached "$TS/dist" && [[ -s "$TS/dist/file.txt.asc" ]] && pass "sign: detached signature written" || fail "sign: detached run failed: $(cat "$TS/out")"
  [[ -z "$(ls -A "$TS/rt")" ]] && pass "sign: nothing left in RUNNER_TEMP after a run (GNUPGHOME and passphrase file gone)" || fail "sign: left in RUNNER_TEMP: $(ls -A "$TS/rt")"
  S nonsense "$TS/dist" && fail "sign: a bad mode must fail" || pass "sign: a bad mode fails"
  [[ -z "$(ls -A "$TS/rt")" ]] && pass "sign: nothing left after a FAILED run either" || fail "sign: left after failure: $(ls -A "$TS/rt")"
  RUNNER_TEMP="$TS/rt" GPG_PRIVATE_KEY="not a key" "$ROOT/actions/sign-files/sign.sh" detached "$TS/dist" >/dev/null 2>&1 && fail "sign: a broken key must fail" || pass "sign: a broken key fails"
  [[ -z "$(ls -A "$TS/rt")" ]] && pass "sign: nothing left after a broken key" || fail "sign: left after broken key: $(ls -A "$TS/rt")"
  grep -q 'trap cleanup EXIT' "$ROOT/actions/sign-files/sign.sh" && pass "sign: sign.sh has the EXIT trap" || fail "sign: EXIT trap missing"
fi
