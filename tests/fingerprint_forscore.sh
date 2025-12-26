#!/bin/bash
# Generate a fingerprint of forScore's data structures
# Run this after forScore updates to capture changes

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FINGERPRINTS_DIR="$SCRIPT_DIR/fingerprints"
FORSCORE_DB="$HOME/Library/Containers/com.mgsdevelopment.forscore/Data/Library/Preferences/library.4sl"
SYNC_DIR="$HOME/Library/Containers/com.mgsdevelopment.forscore/Data/Library/Preferences/Sync"
FORSCORE_APP="/Applications/forScore.app"

mkdir -p "$FINGERPRINTS_DIR"

# Get forScore version
if [ -d "$FORSCORE_APP" ]; then
    VERSION=$(defaults read "$FORSCORE_APP/Contents/Info" CFBundleShortVersionString 2>/dev/null || echo "unknown")
    BUILD=$(defaults read "$FORSCORE_APP/Contents/Info" CFBundleVersion 2>/dev/null || echo "unknown")
else
    VERSION="unknown"
    BUILD="unknown"
fi

FINGERPRINT_FILE="$FINGERPRINTS_DIR/forscore-${VERSION}.fingerprint"
LATEST_LINK="$FINGERPRINTS_DIR/latest.fingerprint"

echo "Generating fingerprint for forScore $VERSION (build $BUILD)..."

{
    echo "# forScore Fingerprint"
    echo "# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo "# Version: $VERSION"
    echo "# Build: $BUILD"
    echo ""

    # Database schema
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

    # Plist structures
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

} > "$FINGERPRINT_FILE"

# Update latest symlink
ln -sf "$(basename "$FINGERPRINT_FILE")" "$LATEST_LINK"

echo "Fingerprint saved to: $FINGERPRINT_FILE"
echo "Latest symlink updated: $LATEST_LINK"
