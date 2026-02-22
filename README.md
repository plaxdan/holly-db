# holly-db

A portable, searchable knowledge graph for developers and their AI agents. Backed by SQLite with full-text and semantic search. Works with Claude Code, Cursor, Codex, or no agent at all — your knowledge stays with you.

## Install

### From GitHub Releases (recommended)

Download the latest binary for your platform from the [Releases page](https://github.com/plaxdan/holly-db/releases):

| Platform | File |
|----------|------|
| Linux x86_64 | `holly-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` |
| macOS Intel | `holly-vX.Y.Z-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `holly-vX.Y.Z-aarch64-apple-darwin.tar.gz` |

Extract and place the `holly` binary somewhere on your `PATH`:

```bash
tar xzf holly-vX.Y.Z-*.tar.gz
mv holly ~/.local/bin/   # or /usr/local/bin/
```

Each release includes `.sha256` checksums and [Sigstore build attestations](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations) for verification.

### From source

```bash
cargo install --path holly-cli
```

## Quick Start

```bash
holly init                                # initialize database
holly remember "SQLite: zero ops, good enough for 100k nodes"
holly search "SQLite"
holly list --type decision
holly mcp-server                          # start MCP server for AI agents
```

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `holly init` | | Initialize database and download embedding model |
| `holly remember "<text>"` | `mem` | Quick capture (casual mode) |
| `holly record --type <type> "<title>"` | `r` | Structured record |
| `holly search "<query>"` | `s` | Hybrid FTS + semantic search |
| `holly list` | `l` | List nodes |
| `holly get <id>` | `g` | Get node details |
| `holly edit <id>` | `update` | Update a node |
| `holly delete <id>` | `rm` | Delete a node |
| `holly connect <from> <to>` | | Connect two nodes |
| `holly context` | | Export context for AI agents |
| `holly audit` | | Health check |
| `holly stats` | | Statistics |
| `holly reindex` | | Backfill missing vector embeddings |
| `holly event record <type>` | | Record lifecycle event |
| `holly event list` | | List events |
| `holly task create "<title>"` | | Create task |
| `holly task start <id>` | | Start task |
| `holly task complete <id>` | | Complete task |
| `holly run start --task <id>` | | Start a run |
| `holly run complete <id>` | | Complete a run |
| `holly import --from <path>` | | Import from legacy Holly |
| `holly mcp-server` | | Start MCP server (stdio transport) |

## MCP Integration

holly-db exposes all 21 tools as an MCP server over stdio, compatible with Claude Code, Cursor, and any MCP client.

Add to your `.mcp.json`:

```json
{
  "mcpServers": {
    "holly-kg": {
      "command": "holly",
      "args": ["mcp-server"],
      "env": {
        "HOLLY_DB_PATH": "/path/to/your/.holly-db/holly.db",
        "HOLLY_AGENT": "claude-code",
        "HOLLY_LLM": "claude-sonnet-4-6"
      }
    }
  }
}
```

## Database Location

Resolution order:
1. `--db <path>` flag
2. `HOLLY_DB_PATH` env var
3. Walk up directories for `.holly-db/holly.db`
4. `~/.holly-db/holly.db` (global fallback)

## Provenance

holly-db auto-stamps all writes with agent/user/LLM identity:

| Env Var | Description |
|---------|-------------|
| `HOLLY_AGENT` | Agent name (e.g., `claude-code`, `cursor`) |
| `HOLLY_USER` | User name |
| `HOLLY_LLM` | LLM name (e.g., `claude-sonnet-4-6`) |

Auto-detected from `CLAUDE_PROJECT_DIR`, `ANTHROPIC_MODEL`, `CURSOR_MODEL`, etc.

## Privacy

holly-db never phones home, never collects usage data, and never makes network calls except when downloading the embedding model via `holly init`. This is a permanent commitment.

## License

MIT
