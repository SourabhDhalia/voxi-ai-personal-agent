#!/bin/bash
# Helper script to start Voxi MCP Server via SDB tunnel
# Run this on your host machine (PC)

DEVICE_SERIAL=$(sdb devices | grep -v "List" | head -n 1 | awk '{print $1}')

if [ -z "$DEVICE_SERIAL" ]; then
    echo "Error: No Voxi device/emulator found via sdb."
    exit 1
fi

echo "Connecting to Voxi MCP Server on $DEVICE_SERIAL..."
echo "Press Ctrl+C to stop."

# Run voxi in MCP stdio mode via sdb shell.
# stdio will be piped through sdb.
sdb -s "$DEVICE_SERIAL" shell "/usr/bin/voxi --mcp-stdio"

