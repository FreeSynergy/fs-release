# Signing with OpenPGP

Every artefact gets a detached, ASCII-armoured signature (`file.asc`), and so does `SHA256SUMS.txt`.
`.deb` (dpkg-sig) and `.rpm` (rpmsign) additionally carry their native package signature. Signing steps run
**only if the secret `GPG_PRIVATE_KEY` is set**; without it the workflow prints a warning and ships unsigned files.

The key is created and kept by the owner. Nothing in this repo generates one.

## Create the key (once, on an offline machine)

```sh
export GNUPGHOME=$(mktemp -d)
gpg --quick-generate-key "FreeSynergy Release <release@freesynergy.net>" ed25519 sign 3y
gpg --list-secret-keys --keyid-format long        # note the fingerprint
```

Ed25519 with a 3 year expiry; extending it later is a one-minute job and does not change the key.
Set a strong passphrase (the CI needs it, see below).

## Revocation certificate and backup (offline)

```sh
FPR=<fingerprint>
gpg --gen-revoke "$FPR" > revoke-FreeSynergy-release.asc     # reason: "key was compromised" or "no longer used"
gpg --armor --export-secret-keys "$FPR" > FreeSynergy-release-secret.asc
gpg --armor --export "$FPR"            > FreeSynergy-release-public.asc
```

Store the secret key and the revocation certificate on offline media (two copies, different places) and
the passphrase in the owner's password manager. Never commit any of them. If the key leaks: import the
revocation certificate, publish it, remove the secrets from GitHub, generate a new key.

## Give it to GitHub

Organisation level (recommended, one place for all repos): *Settings > Secrets and variables > Actions*

| Secret | Content |
|--------|---------|
| `GPG_PRIVATE_KEY` | full text of `FreeSynergy-release-secret.asc` |
| `GPG_PASSPHRASE` | the passphrase (empty is allowed but discouraged) |

Limit the secrets to the repositories that release. The caller workflows pass them explicitly; fs-release only uses
them in the `finalize` job, never in jobs that run build scripts of the app.
(A dedicated signing subkey that is exported without the primary key is an option for later: `gpg --export-secret-subkeys`.)

## Publish the public key

Put `FreeSynergy-release-public.asc` and its fingerprint on freesynergy.net and in the org README. Every release also contains
`FreeSynergy-release-key.pgp` (exported from the secret key in CI), but users must compare the fingerprint with the website.

## Verify as a user

```sh
gpg --import FreeSynergy-release-public.asc
gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt && sha256sum -c SHA256SUMS.txt --ignore-missing
gpg --verify tool-1.0.0-linux-x86_64.tar.gz.asc tool-1.0.0-linux-x86_64.tar.gz
dpkg-sig --verify tool_1.0.0-1_amd64.deb      # needs dpkg-sig
rpm --import FreeSynergy-release-public.asc && rpm -K tool-1.0.0-1.x86_64.rpm
gh attestation verify tool-1.0.0-linux-x86_64.tar.gz --repo FreeSynergy/<repo>
```
