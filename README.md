# holly-db

A portable, searchable knowledge graph for developers and their AI agents. Backed by SQLite with full-text and semantic search.

## Quick Start

```bash
holly init
holly remember "Use SQLite for storage — zero ops, good enough for 100k nodes"
holly search "SQLite"
holly list --type decision
```

## Install

```bash
cargo install --path holly-cli
```

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `holly init` | | Initialize database |
| `holly remember "<text>"` | `mem` | Quick capture |
| `holly record --type <type> "<title>"` | `r` | Structured record |
| `holly search "<query>"` | `s` | Hybrid FTS + semantic search |
| `holly list` | `l` | List nodes |
| `holly get <id>` | `g` | Get node details |
| `holly edit <id>` | `update` | Update a node |
| `holly delete <id>` | `rm` | Delete a node |
| `holly connect <from> <to>` | | Connect two nodes |
| `holly context` | | Export context for agents |
| `holly audit` | | Health check |
| `holly stats` | | Statistics |
| `holly event record <type>` | | Record lifecycle event |
| `holly event list` | | List events |
| `holly task create "<title>"` | | Create task |
| `holly task start <id>` | | Start task |
| `holly task complete <id>` | | Complete task |
| `holly run start --task <id>` | | Start a run |
| `holly run complete <id>` | | Complete a run |
| `holly import --from <path>` | | Import from legacy Holly |

## Database Location

Resolution order:
1. `--db <path>` flag
2. `HOLLY_DB_PATH` env var
3. Walk up directories for `.holly-db/holly.db`
4. `~/.holly-db/holly.db` (global fallback)

## Provenance

holly-db auto-stamps all writes with agent/user/LLM identity from environment variables:

- `HOLLY_AGENT` — agent name override
- `HOLLY_USER` — user name
- `HOLLY_LLM` — LLM name override

Auto-detected from: `CLAUDE_PROJECT_DIR`, `CURSOR_AGENT`, `ANTHROPIC_MODEL`, `CURSOR_MODEL`, etc.

## License

MIT
