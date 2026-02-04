#!/bin/sh
set -e

# Setup Claude credentials symlinks if the config directory exists on the volume
if [ -d "/repos/.claude-config" ]; then
    # For root user (SSH sessions)
    if [ "$(id -u)" = "0" ]; then
        ln -sf /repos/.claude-config /root/.claude
        if [ -f "/repos/.claude-config/.claude.json" ]; then
            ln -sf /repos/.claude-config/.claude.json /root/.claude.json
        fi
    fi

    # For appuser (server process)
    if [ -d "/home/appuser" ]; then
        ln -sf /repos/.claude-config /home/appuser/.claude 2>/dev/null || true
        if [ -f "/repos/.claude-config/.claude.json" ]; then
            ln -sf /repos/.claude-config/.claude.json /home/appuser/.claude.json 2>/dev/null || true
        fi
    fi
fi

# Execute the main command
exec "$@"
