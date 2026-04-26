#!/usr/bin/env bash
# bump-versions.sh — Bump the patch version of all publishable tempoch crates.
#
# This script is called by the automated Monday data-refresh publish workflow
# (update-time-data.yml) after a successful table regeneration.  It reads the
# current version from each crate's Cargo.toml, increments the patch component
# (X.Y.Z → X.Y.Z+1), and rewrites both the crate's own `version =` field and
# any downstream dependency version constraints that reference the old version.
#
# Crate dependency graph (in publish order):
#   tempoch-time-data          (independent versioning track, 0.1.x)
#   tempoch-core               depends on tempoch-time-data
#   tempoch                    depends on tempoch-core
#   tempoch-ffi                depends on tempoch
#
# Usage (from workspace root):
#   bash .github/scripts/bump-versions.sh
#
# Output:
#   Prints the new versions for each crate.
#   Exits non-zero if any Cargo.toml cannot be parsed or updated.
#
# Requirements: GNU sed (available on ubuntu-latest).

set -euo pipefail

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

# Read the first `version = "X.Y.Z"` line from a Cargo.toml.
read_version() {
    local path="$1"
    grep -m1 '^version\s*=' "$path" \
        | sed 's/.*version\s*=\s*"\([^"]*\)".*/\1/'
}

# Increment the patch component of a semver string.
bump_patch() {
    echo "$1" | awk -F. '{printf "%s.%s.%d\n", $1, $2, $3+1}'
}

# Replace the crate's own version declaration (first occurrence only).
# GNU sed `0,/pattern/` limits the substitution to the first match, so we
# do not accidentally replace a dependency constraint of the same version.
set_own_version() {
    local path="$1" old="$2" new="$3"
    sed -i "0,/^version\s*=\s*\"${old}\"/{s/^version\s*=\s*\"${old}\"/version = \"${new}\"/}" "$path"
}

# Replace the version constraint on a named dependency inside a Cargo.toml.
# Matches lines of the form:
#   dep-name = { ..., version = "OLD", ... }
# and replaces only the version value.
set_dep_version() {
    local path="$1" dep="$2" old="$3" new="$4"
    sed -i "s/\(${dep}[^=]*=.*version\s*=\s*\"\)${old}\"/\1${new}\"/" "$path"
}

# ---------------------------------------------------------------------------
# Read current versions
# ---------------------------------------------------------------------------

TD_OLD=$(read_version tempoch-time-data/Cargo.toml)
MAIN_OLD=$(read_version tempoch/Cargo.toml)

# ---------------------------------------------------------------------------
# Compute new versions
# ---------------------------------------------------------------------------

TD_NEW=$(bump_patch "$TD_OLD")
MAIN_NEW=$(bump_patch "$MAIN_OLD")

echo "tempoch-time-data:  ${TD_OLD}   → ${TD_NEW}"
echo "tempoch-core:       ${MAIN_OLD} → ${MAIN_NEW}"
echo "tempoch:            ${MAIN_OLD} → ${MAIN_NEW}"
echo "tempoch-ffi:        ${MAIN_OLD} → ${MAIN_NEW}"

# ---------------------------------------------------------------------------
# Rewrite Cargo.toml files
# ---------------------------------------------------------------------------

# tempoch-time-data: own version only
set_own_version tempoch-time-data/Cargo.toml "$TD_OLD" "$TD_NEW"

# tempoch-core: own version + tempoch-time-data dependency
set_own_version tempoch-core/Cargo.toml "$MAIN_OLD" "$MAIN_NEW"
set_dep_version tempoch-core/Cargo.toml "tempoch-time-data" "$TD_OLD" "$TD_NEW"

# tempoch: own version + tempoch-core dependency
set_own_version tempoch/Cargo.toml "$MAIN_OLD" "$MAIN_NEW"
set_dep_version tempoch/Cargo.toml "tempoch-core" "$MAIN_OLD" "$MAIN_NEW"

# tempoch-ffi: own version + tempoch dependency
set_own_version tempoch-ffi/Cargo.toml "$MAIN_OLD" "$MAIN_NEW"
set_dep_version tempoch-ffi/Cargo.toml "tempoch" "$MAIN_OLD" "$MAIN_NEW"

# ---------------------------------------------------------------------------
# Export new versions for downstream workflow steps
# ---------------------------------------------------------------------------

# Write to GITHUB_OUTPUT if running in a GitHub Actions context.
if [ -n "${GITHUB_OUTPUT:-}" ]; then
    echo "td_version=${TD_NEW}"     >> "$GITHUB_OUTPUT"
    echo "main_version=${MAIN_NEW}" >> "$GITHUB_OUTPUT"
fi

echo "Version bump complete."
