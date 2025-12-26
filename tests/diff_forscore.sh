#!/bin/bash
# Compare current forScore data structures against saved fingerprint
# Usage: ./diff_forscore.sh [fingerprint-file]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FINGERPRINTS_DIR="$SCRIPT_DIR/fingerprints"

# Use provided fingerprint or latest
if [ -n "$1" ]; then
    BASELINE="$1"
else
    BASELINE="$FINGERPRINTS_DIR/latest.fingerprint"
fi

if [ ! -f "$BASELINE" ]; then
    echo "Error: Baseline fingerprint not found: $BASELINE"
    echo "Run ./fingerprint_forscore.sh first to create one"
    exit 1
fi

# Generate current fingerprint to temp file
CURRENT=$(mktemp)
trap "rm -f $CURRENT" EXIT

# Run fingerprint script but capture to temp
FORSCORE_DB="$HOME/Library/Containers/com.mgsdevelopment.forscore/Data/Library/Preferences/library.4sl"
SYNC_DIR="$HOME/Library/Containers/com.mgsdevelopment.forscore/Data/Library/Preferences/Sync"
FORSCORE_APP="/Applications/forScore.app"

if [ -d "$FORSCORE_APP" ]; then
    VERSION=$(defaults read "$FORSCORE_APP/Contents/Info" CFBundleShortVersionString 2>/dev/null || echo "unknown")
    BUILD=$(defaults read "$FORSCORE_APP/Contents/Info" CFBundleVersion 2>/dev/null || echo "unknown")
else
    VERSION="unknown"
    BUILD="unknown"
fi

{
    echo "# forScore Fingerprint"
    echo "# Generated: CURRENT"
    echo "# Version: $VERSION"
    echo "# Build: $BUILD"
    echo ""

    echo "=== DATABASE SCHEMA ==="
    echo ""
    if [ -f "$FORSCORE_DB" ]; then
        sqlite3 "$FORSCORE_DB" ".schema" | grep -E "^CREATE TABLE" | sort
        echo ""
        echo "=== INDEXES ==="
        echo ""
        sqlite3 "$FORSCORE_DB" ".schema" | grep -E "^CREATE INDEX" | sort
        echo ""
        echo "=== ENTITY TYPES ==="
        echo ""
        sqlite3 "$FORSCORE_DB" "SELECT Z_ENT, Z_NAME, Z_SUPER FROM Z_PRIMARYKEY ORDER BY Z_ENT;"
    else
        echo "# Database not found"
    fi
    echo ""

    echo "=== SETLIST (.set) STRUCTURE ==="
    echo ""
    SET_FILE=$(ls "$SYNC_DIR"/*.set 2>/dev/null | head -1)
    if [ -n "$SET_FILE" ]; then
        echo "# Sample file: $(basename "$SET_FILE")"
        gunzip -c "$SET_FILE" | plutil -p - | grep -E '^\s+"[^"]+"\s+=>' | sed 's/=>.*/=>/' | sort -u
        echo ""
        echo "# Items entry structure:"
        gunzip -c "$SET_FILE" | plutil -p - | grep -A1 '0 =>' | grep -E '^\s+"[^"]+"\s+=>' | sed 's/=>.*/=>/' | sort -u | head -10
    else
        echo "# No .set files found"
    fi
    echo ""

    echo "=== ITM (.itm) STRUCTURE ==="
    echo ""
    ITM_FILE=$(ls "$SYNC_DIR"/*.itm 2>/dev/null | head -1)
    if [ -n "$ITM_FILE" ]; then
        echo "# Sample file: $(basename "$ITM_FILE")"
        echo "# Top-level keys:"
        gunzip -c "$ITM_FILE" | plutil -p - | grep -E '^  "[^"]+"\s+=>' | sed 's/=>.*/=>/' | sort -u
    else
        echo "# No .itm files found"
    fi
    echo ""

    echo "=== FOLDER (.fld) STRUCTURE ==="
    echo ""
    FLD_FILE=$(ls "$SYNC_DIR"/*.fld 2>/dev/null | head -1)
    if [ -n "$FLD_FILE" ]; then
        echo "# Sample file: $(basename "$FLD_FILE")"
        gunzip -c "$FLD_FILE" | plutil -p - | grep -E '^\s+"[^"]+"\s+=>' | sed 's/=>.*/=>/' | sort -u
    else
        echo "# No .fld files found"
    fi
    echo ""

    echo "=== ZCYLON SAMPLE ==="
    echo ""
    if [ -f "$FORSCORE_DB" ]; then
        echo "# Column order: Z_PK|Z_ENT|Z_OPT|ZSHUFFLE|ZITEM|Z4_ITEM|ZSETLIST|Z_FOK_SETLIST|ZUUID"
        sqlite3 "$FORSCORE_DB" "SELECT * FROM ZCYLON LIMIT 3;"
    fi
    echo ""

    echo "=== ZSETLIST SAMPLE ==="
    echo ""
    if [ -f "$FORSCORE_DB" ]; then
        echo "# Columns:"
        sqlite3 "$FORSCORE_DB" "PRAGMA table_info(ZSETLIST);" | cut -d'|' -f2 | tr '\n' '|'
        echo ""
        sqlite3 "$FORSCORE_DB" "SELECT Z_PK, Z_ENT, Z_OPT, ZINDEX, ZTITLE FROM ZSETLIST LIMIT 3;"
    fi

} > "$CURRENT"

# Extract baseline version
BASELINE_VERSION=$(grep "^# Version:" "$BASELINE" | cut -d: -f2 | tr -d ' ')

echo "Comparing forScore $VERSION against baseline $BASELINE_VERSION"
echo ""

# Filter out timestamps, sample data, and version-specific lines for comparison
filter_for_diff() {
    grep -v "^# Generated:" | \
    grep -v "^# Sample file:" | \
    grep -v "^=== .* SAMPLE ===" | \
    grep -v "^# Column order:" | \
    grep -v "^# Columns:" | \
    grep -vE "^\d+\|" | \
    grep -v "^$"
}

BASELINE_FILTERED=$(mktemp)
CURRENT_FILTERED=$(mktemp)
trap "rm -f $CURRENT $BASELINE_FILTERED $CURRENT_FILTERED" EXIT

filter_for_diff < "$BASELINE" > "$BASELINE_FILTERED"
filter_for_diff < "$CURRENT" > "$CURRENT_FILTERED"

if diff -q "$BASELINE_FILTERED" "$CURRENT_FILTERED" > /dev/null 2>&1; then
    echo "✓ No structural changes detected"
    exit 0
else
    echo "⚠ Changes detected:"
    echo ""
    diff -u "$BASELINE_FILTERED" "$CURRENT_FILTERED" || true
    exit 1
fi
