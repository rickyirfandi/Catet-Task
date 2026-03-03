# Catet Task

Lightweight Jira time tracker built with Tauri + Svelte.

`Catet Task` is a tray-first desktop app focused on macOS workflows: start/pause/resume task timers, review today or weekly totals, and push worklogs to Jira with local-first persistence.

Version: `0.1.0`

## Highlights

- Tray app with fast Timer / Today / Weekly / Settings tabs.
- Jira API token login with auto-verify on app start.
- One active timer model with pause/resume support.
- Robust timer recovery for:
  - laptop sleep / wake,
  - force close / crash + reopen,
  - long inactive gaps.
- Local-timezone based grouping for Today and Weekly views.
- Task detail view with richer metadata (type, priority, assignee, created/updated, description).
- Daily reminder notifications with configurable time picker and presets (`16:00`, `17:00`, `18:00`).
- Export options:
  - copy report text to clipboard,
  - download CSV.

## Tech Stack

- Frontend: Svelte 5 + TypeScript + Vite
- Desktop shell: Tauri v2 (Rust)
- Local storage: SQLite (`sqlx`)
- Notifications: `tauri-plugin-notification`
- Launch at login: `tauri-plugin-autostart`

## Requirements

- Node.js 20+
- npm 10+
- Rust stable toolchain
- Tauri v2 prerequisites for your OS

macOS specific:

- Xcode Command Line Tools installed
- App notifications allowed in System Settings

## Quick Start

```bash
npm install
npm run tauri dev
```

Useful commands:

| Command | Purpose |
| --- | --- |
| `npm run dev` | Run frontend only (Vite) |
| `npm run build` | Build frontend assets |
| `npm run tauri dev` | Run full desktop app in dev mode |
| `npm run tauri build` | Build production desktop app |
| `cargo check` (in `src-tauri`) | Validate Rust compile |

## Build (macOS Example)

```bash
npm run build
npm run tauri build -- --target aarch64-apple-darwin
```

## How Robust Timer Handling Works

The app uses layered protection so tracked time is not inflated by sleep/offline gaps and survives unexpected app closure.

1. Persisted timer session:
- Active timer state is stored in settings (`timer_session_v1`) so it can be restored on relaunch.

2. Heartbeat:
- While running, backend writes a UTC heartbeat (`timer_heartbeat_utc_v1`) every ~5s.
- On restart, this heartbeat is used to estimate external downtime and exclude it.

3. Inactive-gap compensation:
- Runtime tick logic detects long gaps (`>20s`) and shifts timer start to remove inactive duration.

4. macOS power notifications:
- On sleep: engine marks sleep start.
- On wake: exact sleep duration is excluded from active timer.

5. DB recovery:
- Open/unclosed entries are reconciled during startup recovery to avoid duplicate active sessions.

## Timezone Behavior

- Today view queries by local date (`date(start_time, 'localtime')`).
- Weekly view uses local Monday-Sunday range.
- Task detail segment times are rendered in local timezone.
- Reminder time is interpreted and displayed in local Mac timezone.

## Jira Auth and Security

- Primary credential storage: OS keychain (`keyring`).
- Fallback for unsigned dev builds: encrypted credential blob in local settings.
- Jira API calls use Basic auth over HTTPS.

## Data Persistence

- Local database file: `catet-task.db` in Tauri app config directory.
- App updates via new DMG typically keep data if app config directory is preserved.
- "Reset Data" in Settings removes time entries but keeps task cache/settings where applicable.

## Export Format

Clipboard export produces a report-friendly plain text block:

- report title
- period
- generation timestamp
- total tracked time
- per-task breakdown with percentage

CSV export includes:

- `Task ID`
- `Summary`
- `Duration`

## Project Structure

| Path | Description |
| --- | --- |
| `src/` | Svelte frontend |
| `src/components/` | Main UI screens and shared components |
| `src/lib/stores/` | Frontend state stores |
| `src-tauri/src/commands/` | Tauri command handlers |
| `src-tauri/src/timer/` | Timer engine and heartbeat logic |
| `src-tauri/src/power.rs` | macOS sleep/wake integration |
| `src-tauri/src/reminder/` | Daily reminder scheduler |
| `src-tauri/migrations/` | SQLite schema migration |

## Troubleshooting

### Reminder notification does not fire

- Confirm `Daily Reminder` is enabled in Settings.
- Confirm reminder time and timezone are correct.
- Ensure macOS notification permission is granted for the app.
- Keep the app running (tray) for scheduled checks.

### `vite build` fails with `spawn EPERM` / esbuild

- Close editors or antivirus tools that may lock `node_modules`.
- Remove and reinstall dependencies:

```bash
rm -rf node_modules package-lock.json
npm install
```

- On macOS, remove quarantine flags if needed:

```bash
xattr -dr com.apple.quarantine node_modules
```

- Re-run:

```bash
npm run build
```

## Contributing

1. Create a feature branch.
2. Run checks before PR:
- `npm run build` (frontend)
- `cargo check` (backend)
3. Include reproduction steps for bug fixes involving timer/sleep/restart behavior.

## License

No license file is currently included in this repository.
