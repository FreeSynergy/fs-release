## Verify your download

- Checksums: `sha256sum -c SHA256SUMS.txt`
- OpenPGP: `gpg --import FreeSynergy-release-key.pgp && gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt`
  (every artefact has its own `.asc`; compare the key fingerprint with the one on freesynergy.net)
- Build provenance (SLSA): `gh attestation verify <file> --repo ${REPO}`
- SBOM: `*.spdx.json` (SPDX 2.3)

## macOS and Windows

These builds are not signed with a paid Apple or Microsoft certificate.
On macOS open the app once, then System Settings > Privacy & Security > "Open Anyway"
(details in `OPEN-ANYWAY.txt` inside the download). On Windows choose "More info" > "Run anyway"
in SmartScreen. The OpenPGP signatures and checksums above are the trust path.

