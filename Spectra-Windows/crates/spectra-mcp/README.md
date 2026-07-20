# spectra-mcp

MCP server exposing Spectra's full command surface (workspaces, requests,
environments, folders, history, saved responses, import/export) to any
MCP-speaking AI agent, over stdio. It operates on the exact same `~/.spectra`
state as the GUI — nothing here is agent-only or GUI-only.

## Build

```bash
cargo build --release -p spectra-mcp
# binary at target/release/spectra-mcp
```

## Register with Claude Code

```bash
./scripts/install-mcp.sh
# or manually:
claude mcp add spectra -- /absolute/path/to/spectra-app/target/release/spectra-mcp
```

Verify with `claude mcp list`, then ask Claude to e.g. "list my Spectra workspaces."

## Register with Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

```json
{
  "mcpServers": {
    "spectra": {
      "command": "/absolute/path/to/spectra-app/target/release/spectra-mcp"
    }
  }
}
```

Restart Claude Desktop after editing. `./scripts/install-mcp.sh --print-config`
prints this snippet with the path already filled in for your machine.

## Notes

- No environment variables or CLI flags needed — it reads/writes `~/.spectra`
  directly, identical to the Tauri GUI.
- Requires the release binary to exist at a stable path; if you move the repo,
  rebuild and re-run the install step (`claude mcp remove spectra` first if
  re-adding under the same name).
