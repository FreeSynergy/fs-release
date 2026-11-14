# Signing with OpenPGP

Every artefact gets a detached, ASCII-armoured signature (`file.asc`), and so does `SHA256SUMS.txt`.
`.deb` (dpkg-sig) and `.rpm` (rpmsign) additionally carry their native package signature. Signing steps run
**only if the secret `GPG_PRIVATE_KEY` is set**; without it the workflow prints a warning and ships unsigned files.

The key is created and kept by the owner. Nothing in this repo generates one.

## Create the key (once, on an offline machine)

```sh
export GNUPGHOME=$(mktemp -d)
gpg --quick-generate-key "FreeSynergy Release <release@freesynergy.net>" ed25519 cert 3y   # primary key: certify only, stays offline
gpg --quick-add-key <primary-fingerprint> ed25519 sign 1y                                    # signing subkey, the only key CI gets
gpg --list-secret-keys --keyid-format long --with-subkey-fingerprints                        # note both fingerprints
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
| `GPG_PRIVATE_KEY` | export of the signing subkey ONLY: `gpg --armor --export-secret-subkeys <subkey-fingerprint>!` (the primary key never goes to CI) |
| `GPG_PASSPHRASE` | the passphrase (empty is allowed but discouraged) |

Limit the secrets to the repositories that release. The caller workflows pass them explicitly; fs-release only uses
them in the `finalize` job, never in jobs that run build scripts of the app.
The signing subkey without the primary key IS in use now: `actions/sign-apt` forces exactly one subkey (`signing-subkey` input, default `372A583D05BF8362604CF25DE26A2347F2B1C097`, with `!`) and fails if the secret does not contain it. Keep the offline backup of the full secret key as described above; only the subkey export goes into the secret. The subkey expires 2027-11-07: extend it offline, export it again and update the secret before then.

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

## APT repositories (`actions/sign-apt`)

`.deb` files carry no signature that `apt` checks; trust comes from the repository metadata. The action `FreeSynergy/fs-release/actions/sign-apt`
signs `dists/<suite>/Release` of every suite below `directory` as `InRelease` (clear-signed) and `Release.gpg` (detached, armoured) with the same
secrets (`GPG_PRIVATE_KEY`, `GPG_PASSPHRASE`; the signing subkey alone is enough). Unlike `sign-files` it fails without a key. The tree itself is built
in the `fs-packages` repository (`scripts/build-apt.sh`, `apt-ftparchive`); users check it with `scripts/verify-apt.sh` or simply with `apt update`.
Use it only in the signing job, pinned by commit SHA.
