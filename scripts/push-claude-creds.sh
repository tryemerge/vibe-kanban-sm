#!/bin/bash
# Push local Claude credentials to Fly.io container
# Usage: ./scripts/push-claude-creds.sh [app-name]

APP_NAME="${1:-vibe-kanban-sm}"

echo "Pushing Claude credentials to $APP_NAME..."

# Create temp directory for the files
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Copy credentials to temp
cp ~/.claude/.credentials.json "$TEMP_DIR/" 2>/dev/null || true
cp ~/.claude.json "$TEMP_DIR/" 2>/dev/null || true

# Check we have at least one file
if [ ! -f "$TEMP_DIR/.credentials.json" ] && [ ! -f "$TEMP_DIR/.claude.json" ]; then
    echo "Error: No credentials found at ~/.claude/.credentials.json or ~/.claude.json"
    exit 1
fi

# Push to Fly via SSH
echo "Uploading credentials..."

if [ -f "$TEMP_DIR/.credentials.json" ]; then
    cat "$TEMP_DIR/.credentials.json" | fly ssh console -a "$APP_NAME" -C "cat > /repos/.claude-config/.credentials.json"
    echo "  ✓ .credentials.json uploaded"
fi

if [ -f "$TEMP_DIR/.claude.json" ]; then
    cat "$TEMP_DIR/.claude.json" | fly ssh console -a "$APP_NAME" -C "cat > /repos/.claude-config/.claude.json"
    echo "  ✓ .claude.json uploaded"
fi

# Fix permissions
fly ssh console -a "$APP_NAME" -C "chown -R appuser:appgroup /repos/.claude-config 2>/dev/null || true"

echo "Done! Credentials pushed to $APP_NAME"
