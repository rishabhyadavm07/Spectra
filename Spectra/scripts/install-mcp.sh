#!/usr/bin/env bash
# Builds spectra-mcp in release mode and registers it with an MCP client.
#
# Usage:
#   scripts/install-mcp.sh                 # registers with Claude Code (claude mcp add)
#   scripts/install-mcp.sh --print-config  # just prints the Claude Desktop config snippet
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

BIN_PATH="$(pwd)/target/release/spectra-mcp"

echo "Building spectra-mcp (release)..."
cargo build --release -p spectra-mcp

if [[ "${1:-}" == "--print-config" ]]; then
  cat <<EOF
Add this to your Claude Desktop config (~/Library/Application Support/Claude/claude_desktop_config.json on macOS):

{
  "mcpServers": {
    "spectra": {
      "command": "${BIN_PATH}"
    }
  }
}
EOF
  exit 0
fi

if ! command -v claude >/dev/null 2>&1; then
  echo "claude CLI not found on PATH. Install Claude Code, or rerun with --print-config" \
       "for a Claude Desktop config snippet instead." >&2
  exit 1
fi

echo "Registering spectra with Claude Code..."
claude mcp add spectra -- "${BIN_PATH}"

echo "Done. Verify with: claude mcp list"
