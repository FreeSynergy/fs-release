#!/usr/bin/env bash
# Pattern: Strategy - one function per native Linux package format, each building ONE cargo package.
# Source it after scripts/lib.sh; package.sh (Template method) calls each strategy once per group from package_groups.
# Contract (all need $PLAN, the workspace as cwd; DIST is an existing output directory):
#   deb_package PKG TRIPLE DIST        cargo deb for PKG (binaries must already be built for TRIPLE); writes into DIST.
#   rpm_package PKG BINS TRIPLE DIST   cargo generate-rpm for PKG; BINS is the comma list of PKG's binaries, used for the
#                                      default assets when the package has no [package.metadata.generate-rpm]; writes into DIST.
set -euo pipefail

deb_package() { die "deb_package: not implemented"; }
rpm_package() { die "rpm_package: not implemented"; }
