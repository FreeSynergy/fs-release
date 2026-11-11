#!/usr/bin/env bash
# Pattern: none. Installs the cargo tools named in $TOOLS with cargo-binstall (already installed by the caller step).
# Shared by actions/install-tools and (through a path, not a `uses: ...@main`) by actions/sbom and actions/licence-check,
# so a pinned action never pulls a later main state of another action.
set -euo pipefail
# shellcheck disable=SC2086  # word splitting of the tool list is intended
cargo binstall --no-confirm --locked $TOOLS
