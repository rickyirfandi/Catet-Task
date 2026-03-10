<div align="center">
  <img src="jtt logo new 2.png" alt="Catet Task" width="128" />
  <h1>Catet Task</h1>
  <p><strong>A lightweight, local-first Jira time tracker that lives in your menu bar.</strong></p>
  <p>
    <a href="#features">Features</a> &bull;
    <a href="#download">Download</a> &bull;
    <a href="#building-from-source">Build</a> &bull;
    <a href="#cli-and-mcp-claude-integration">CLI + MCP</a> &bull;
    <a href="#contributing">Contributing</a> &bull;
    <a href="#license">License</a>
  </p>
  <p>
    <img alt="Platform" src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows-informational" />
    <img alt="Built with Tauri" src="https://img.shields.io/badge/built%20with-Tauri%20v2-blue" />
    <img alt="License" src="https://img.shields.io/badge/license-MIT-green" />
  </p>
</div>

---

Catet Task is a tray-first desktop app for tracking time on Jira tasks. Start, pause, and resume timers from the system tray, review daily or weekly totals, and push worklogs to Jira — all with local-first persistence so your data is never lost.

## Features

- **Tray-native** — lives in the menu bar / system tray with real-time elapsed time display
- **One active timer** — start/pause/resume with automatic switching between tasks
- **Robust timer recovery** — survives sleep, wake, force-quit, and crash with no lost time
- **Partial log flow** — choose which entries to submit; running timers stay untouched
- **Today & Weekly views** — daily summary and weekly breakdown with per-task percentages
- **Task detail** — view issue type, priority, assignee, description, and session history
- **Export** — copy report to clipboard or download as CSV
- **Daily reminders** — configurable notification to remind you to log time
- **Dark theme** — purpose-built dark UI, no light mode
- **Offline-capable** — time tracking works fully offline; only fetch and submit need connectivity
- **CLI + MCP** — automate workflows from terminal and Claude

## Tech Stack

| Layer | Technology |
|---|---|
| App shell | [Tauri v2](https://v2.tauri.app/) (Rust) |
| Frontend | [Svelte 5](https://svelte.dev/) + TypeScript + Vite |
| Local DB | SQLite via [sqlx](https://github.com/launchbadge/sqlx) |
| Credentials | OS keychain via [keyring](https://crates.io/crates/keyring) |
| HTTP | [reqwest](https://crates.io/crates/reqwest) (rustls) |
| Automation | `catet-cli` + MCP server (Model Context Protocol) |

## Download

Pre-built binaries are available from [Releases](../../releases). Builds are produced via GitHub Actions on every tag push.

| Platform | Artifact |
|---|---|
| macOS (Apple Silicon) | `.dmg` |
| macOS (Intel) | `.dmg` |
| Windows (x64) | `.msi` / `.exe` |

> **macOS note:** The app is signed and notarized. If Gatekeeper still warns you, right-click the app and choose "Open".

## Building from Source

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) stable toolchain
- Tauri v2 system dependencies ([macOS](https://v2.tauri.app/start/prerequisites/#macos) | [Windows](https://v2.tauri.app/start/prerequisites/#windows) | [Linux](https://v2.tauri.app/start/prerequisites/#linux))

### Steps

```bash
# Clone the repo
git clone https://github.com/nicvit/catet-task.git
cd catet-task

# Install frontend dependencies
npm install

# (Optional) Build CLI standalone binary
cargo build --manifest-path catet-cli/Cargo.toml --release

# Run in development mode (hot-reload)
npm run tauri dev

# Build for production
npm run tauri build
```

### Useful Commands

| Command | Purpose |
|---|---|
| `npm run dev` | Frontend only (Vite dev server) |
| `npm run build` | Build frontend assets |
| `npm run tauri dev` | Full app in dev mode |
| `npm run tauri build` | Production build |
| `cargo build --manifest-path catet-cli/Cargo.toml --release` | Build CLI binary |
| `catet-cli --help` | List CLI commands |
| `catet-cli serve-mcp` | Run MCP server over stdio |
| `cd src-tauri && cargo check` | Validate Rust compilation |
| `cd src-tauri && cargo clippy` | Lint Rust code |
| `cd src-tauri && cargo fmt` | Format Rust code |

## How It Works

### Timer Engine

The timer runs entirely in Rust — not in the browser. A `tokio` interval ticks every second, updates the tray title, and emits events to the Svelte frontend. This means:

- Closing the panel window does **not** stop the timer
- Timer state is persisted to SQLite so it survives app restarts
- Sleep/wake cycles are detected and excluded from tracked time
- A heartbeat mechanism estimates downtime after unexpected shutdowns

### Jira Integration

- **Auth:** API token stored in the OS keychain; Basic auth over HTTPS
- **Tasks:** Fetched via JQL (`assignee = currentUser() AND status IN ("To Do", "In Progress") AND sprint IN openSprints()`)
- **Worklogs:** POSTed to `/rest/api/3/issue/{key}/worklog` with Atlassian Document Format comments
- **Error handling:** 401 triggers re-login, 429 retries with exponential backoff, 5xx retries up to 3 times

### Data Storage

All data lives locally in a SQLite database at the Tauri app config directory:

| OS | Path |
|---|---|
| macOS | `~/Library/Application Support/id.rickyirfandi.catettask/catet-task.db` |
| Windows | `%APPDATA%/id.rickyirfandi.catettask/catet-task.db` |
| Linux | `~/.local/share/id.rickyirfandi.catettask/catet-task.db` |

Entries are never hard-deleted — soft flags preserve history for reporting.

## CLI and MCP (Claude Integration)

Catet Task ships with a companion CLI, `catet-cli`, that can be used directly in Terminal and as an MCP server for Claude clients.

### What the CLI provides

- Local reporting and entry management (`status`, `today`, `week`, `entries`, `set-comment`, `set-duration`, `submit`, `report`)
- Task lookup and search automation (`tasks`, `catet_search_tasks` through MCP)
- MCP stdio server via `catet-cli serve-mcp`

### Install location

When installed from app Settings (or `catet-cli install`), default binary path is:

| OS | Path |
|---|---|
| macOS | `~/.local/bin/catet-cli` |
| Windows | `%LOCALAPPDATA%/Programs/catet-cli/catet-cli.exe` |
| Linux | `~/.local/bin/catet-cli` |

### Connect to Claude Desktop

Recommended:

1. Open **Settings** in Catet Task.
2. Install CLI tools (if not installed yet).
3. Click **Connect to Claude Desktop**.
4. Restart Claude Desktop.

Manual config (if needed), in Claude Desktop config:

```json
{
  "mcpServers": {
    "catet-task": {
      "command": "C:\\Users\\<you>\\AppData\\Local\\Programs\\catet-cli\\catet-cli.exe",
      "args": ["serve-mcp"]
    }
  }
}
```

### Connect to Claude Code

```bash
catet-cli connect-claude-code
```

Or manually:

```bash
claude mcp add catet-task "catet-cli" serve-mcp --scope user
```

### MCP tools exposed

- `catet_status`, `catet_today`, `catet_week`, `catet_range`, `catet_entries`
- `catet_set_comment`, `catet_set_duration`
- `catet_submit_preview`, `catet_submit`
- `catet_tasks`, `catet_search_tasks`
- `catet_plan_manual_log`, `catet_add_manual_log`
- `catet_report`

### MCP smoke test

```bash
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"smoke","version":"1"}}}' | catet-cli serve-mcp
```

```powershell
'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"smoke","version":"1"}}}' | catet-cli serve-mcp
```

Expected result includes:

- `protocolVersion: 2025-11-25`
- `serverInfo.name: catet-task`

## Project Structure

```
catet-task/
├── src/                        # Svelte 5 frontend
│   ├── components/             # UI screens and shared components
│   ├── lib/stores/             # Reactive state (auth, tasks, timer, entries)
│   ├── lib/api/                # Type-safe Tauri invoke wrappers
│   └── lib/utils/              # Formatting, rounding helpers
│
├── src-tauri/                  # Rust backend
│   ├── src/commands/           # Tauri command handlers
│   ├── src/jira/               # Jira API client and models
│   ├── src/timer/              # Timer engine and heartbeat
│   ├── src/db/                 # SQLite queries
│   └── migrations/             # SQL schema migrations
│
└── .github/workflows/          # CI: macOS and Windows builds
```

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Cmd/Ctrl + Shift + T` | Toggle panel |
| `Cmd/Ctrl + Shift + P` | Pause / Resume |
| `Cmd/Ctrl + Shift + L` | Open log flow |
| `Cmd/Ctrl + K` | Focus search (panel open) |
| `Escape` | Close panel |

## Troubleshooting

### Reminder notification does not fire

- Confirm **Daily Reminder** is enabled in Settings
- Verify the reminder time and timezone are correct
- Ensure notification permission is granted for the app
- Keep the app running in the tray

### Claude MCP cannot connect / server disconnects

- Confirm CLI command path in Claude config points to a real binary.
- Ensure MCP args include exactly `serve-mcp`.
- Reconnect from app Settings and restart Claude Desktop.
- Run MCP smoke test manually:

```bash
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"smoke","version":"1"}}}' | catet-cli serve-mcp
```

- If CLI was built in debug/dev earlier, reinstall/update CLI from app Settings to avoid stale binaries.

### `vite build` fails with `spawn EPERM` / esbuild

This is usually caused by editors or antivirus tools locking files in `node_modules`:

```bash
rm -rf node_modules package-lock.json
npm install
npm run build
```

On macOS, you may also need to remove quarantine flags:

```bash
xattr -dr com.apple.quarantine node_modules
```

## Contributing

Contributions are welcome! Here's how to get started:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run checks:
   - `npm run build` — frontend compiles
   - `cd src-tauri && cargo clippy` — no warnings
   - `cd src-tauri && cargo fmt --check` — formatting passes
5. Commit with a descriptive message
6. Open a pull request

For bug fixes involving timer, sleep/wake, or restart recovery, please include reproduction steps.

## License

[MIT](LICENSE) &copy; Ricky Irfandi
