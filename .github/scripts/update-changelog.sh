#!/usr/bin/env bash
# update-changelog.sh — Prepend a new release section to CHANGELOG.md.
#
# This script is called by the automated Monday data-refresh publish workflow
# (update-time-data.yml) after bump-versions.sh has updated Cargo.toml files.
# It compares the upstream-data SHA256 hashes stored in the provenance sidecar
# at HEAD against the same file at the last version tag, then inserts a new
# section under `## [Unreleased]` in CHANGELOG.md describing which datasets
# changed.
#
# Usage (from workspace root):
#   bash .github/scripts/update-changelog.sh <new-version>
#
#   Use 'unreleased' as version to update the [Unreleased] section without
#   creating a new versioned header (used when publication is blocked by WIP).
#
#   Example:
#     bash .github/scripts/update-changelog.sh unreleased
#     bash .github/scripts/update-changelog.sh 0.4.3
#
# The script requires:
#   - jq (available on ubuntu-latest)
#   - GNU sed (available on ubuntu-latest)
#   - git with the last annotated version tag reachable as v*.*.*
#
# Provenance fields compared (from time_data.provenance.json):
#   utc_tai_sha256           — UTC-TAI leap second history (BIPM/IERS)
#   delta_t_observed_sha256  — Observed ΔT = TT − UT1 (USNO monthly data)
#   delta_t_predictions_sha256 — Predicted ΔT (USNO)
#   eop_finals_sha256        — Earth Orientation Parameters finals2000A (IERS)

set -euo pipefail

# ---------------------------------------------------------------------------
# Arguments
# ---------------------------------------------------------------------------

if [ $# -ne 1 ]; then
    echo "Usage: $0 <new-version>" >&2
    exit 1
fi

NEW_VERSION="$1"
PROVENANCE_PATH="tempoch-core/src/generated/time_data.provenance.json"
CHANGELOG="CHANGELOG.md"
TODAY=$(date -u +%Y-%m-%d)

# ---------------------------------------------------------------------------
# Load provenance: new (HEAD) and old (last version tag)
# ---------------------------------------------------------------------------

NEW_PROV=$(cat "$PROVENANCE_PATH")

LAST_TAG=$(git describe --tags --abbrev=0 --match "v[0-9]*.[0-9]*.[0-9]*" 2>/dev/null || echo "")

if [ -n "$LAST_TAG" ]; then
    OLD_PROV=$(git show "${LAST_TAG}:${PROVENANCE_PATH}" 2>/dev/null || echo "{}")
else
    # No prior tag — treat every dataset as changed.
    OLD_PROV="{}"
fi

# Helper: extract a field from a JSON string using jq.
jq_field() {
    echo "$1" | jq -r ".\"$2\" // empty"
}

# ---------------------------------------------------------------------------
# Determine which datasets changed
# ---------------------------------------------------------------------------

CHANGED_LINES=""

check_dataset() {
    local field="$1" label="$2"
    local old_val new_val
    old_val=$(jq_field "$OLD_PROV" "$field")
    new_val=$(jq_field "$NEW_PROV" "$field")
    if [ "$old_val" != "$new_val" ]; then
        CHANGED_LINES="${CHANGED_LINES}  - ${label}\n"
    fi
}

check_dataset "utc_tai_sha256"              "UTC-TAI leap second history (BIPM/IERS)"
check_dataset "delta_t_observed_sha256"     "Observed ΔT = TT − UT1 (USNO monthly data)"
check_dataset "delta_t_predictions_sha256"  "Predicted ΔT (USNO)"
check_dataset "eop_finals_sha256"           "Earth Orientation Parameters finals2000A (IERS)"

# Fall back to a generic message if no SHA changed (should not normally occur).
if [ -z "$CHANGED_LINES" ]; then
    CHANGED_LINES="  - Generated time tables refreshed (upstream data content unchanged)\n"
fi

# Fetch timestamp from the new provenance (first 10 characters = YYYY-MM-DD).
FETCH_DATE=$(jq_field "$NEW_PROV" "fetched_utc" | cut -c1-10)

# If in 'unreleased' mode, avoid duplicate entries for the same fetch date.
if [ "$NEW_VERSION" = "unreleased" ] && grep -q "Refreshed generated time tables (fetched $FETCH_DATE)" "$CHANGELOG"; then
    echo "CHANGELOG.md already updated for fetch date $FETCH_DATE. Skipping."
    exit 0
fi

# ---------------------------------------------------------------------------
# Build the new changelog section
# ---------------------------------------------------------------------------

# Written to a temp file so `sed -i` can splice it in cleanly without
# requiring multi-line shell variables.
TMP_ENTRY=$(mktemp /tmp/changelog_entry.XXXXXX)
# Ensure temp file is removed on exit.
trap 'rm -f "$TMP_ENTRY"' EXIT

if [ "$NEW_VERSION" = "unreleased" ]; then
    printf "\n### Changed\n\n- Refreshed generated time tables (fetched %s):\n%s\n" \
        "$FETCH_DATE" "$(printf "%b" "$CHANGED_LINES")" \
        > "$TMP_ENTRY"
else
    printf "\n## [%s] - %s\n\n### Changed\n\n- Refreshed generated time tables (fetched %s):\n%s\n" \
        "$NEW_VERSION" "$TODAY" "$FETCH_DATE" "$(printf "%b" "$CHANGED_LINES")" \
        > "$TMP_ENTRY"
fi

# ---------------------------------------------------------------------------
# Splice the entry into CHANGELOG.md after the ## [Unreleased] heading
# ---------------------------------------------------------------------------
# `sed -i "/pattern/r file"` appends the file content after the matched line.

if ! grep -q "^## \[Unreleased\]" "$CHANGELOG"; then
    echo "Warning: no '## [Unreleased]' heading found in $CHANGELOG; prepending entry." >&2
    cat "$TMP_ENTRY" "$CHANGELOG" > "${CHANGELOG}.tmp" && mv "${CHANGELOG}.tmp" "$CHANGELOG"
else
    sed -i "/^## \[Unreleased\]/r ${TMP_ENTRY}" "$CHANGELOG"
fi

echo "CHANGELOG.md updated with section [${NEW_VERSION}] - ${TODAY}."
