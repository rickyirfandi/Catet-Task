# CLAUDE.md — Catet Task

## Project Overview

Catet Task is a **menu bar / system tray application** for tracking time on Jira tasks. Built with **Rust (Tauri v2)** backend and **Svelte 5** frontend. The app lives in the system tray, shows the active task + elapsed time (e.g., `● ABC-123 · 00:34:12`), and lets users batch-submit worklogs to Jira.

**Key docs:**
- `docs/PRD.docx` — Full product requirements document
- `docs/mockups/` — UI mockup HTML files

---

## Tech Stack

| Layer | Tech | Version |
|-------|------|---------|
| App Shell | Tauri | v2 |
| Backend | Rust | 2024 edition |
| Frontend | Svelte | 5 (Runes) |
| Bundler | Vite | latest |
| Local DB | SQLite | via sqlx (direct) |
| HTTP | reqwest | latest (async, json, rustls-tls) |
| Credentials | keyring | latest |
| Tray | tauri built-in | `tray-icon` feature |
| Positioning | tauri-plugin-positioner | latest |
| Serialization | serde + serde_json | latest |
| Async runtime | tokio | (bundled with Tauri) |

---

## Project Structure

```
catet-task/
├── CLAUDE.md                    # ← You are here
├── README.md
├── package.json
├── svelte.config.js
├── vite.config.ts
├── tsconfig.json
│
├── src/                         # Svelte frontend
│   ├── App.svelte               # Root — router between Login/Main
│   ├── main.ts                  # Entry point
│   ├── app.css                  # Global styles + design tokens
│   │
│   ├── lib/
│   │   ├── stores/
│   │   │   ├── auth.ts          # Auth state (user info, logged in flag)
│   │   │   ├── tasks.ts         # Task list, search, pinned
│   │   │   ├── timer.ts         # Active timer state, elapsed seconds
│   │   │   └── entries.ts       # Today's time entries, selection state
│   │   │
│   │   ├── api/
│   │   │   └── tauri.ts         # Type-safe invoke() wrappers for all Tauri commands
│   │   │
│   │   ├── utils/
│   │   │   ├── time.ts          # formatDuration(), parseSeconds(), etc.
│   │   │   └── rounding.ts      # roundToNearest(seconds, increment)
│   │   │
│   │   └── types.ts             # Shared TypeScript types (mirroring Rust structs)
│   │
│   └── components/
│       ├── Login.svelte          # Auth screen (domain, email, token)
│       ├── Panel.svelte          # Main panel wrapper with tab navigation
│       ├── Timer.svelte          # Timer tab — task list + active timer
│       ├── TaskCard.svelte       # Individual task card with play/pause
│       ├── SearchBar.svelte      # Task search input
│       ├── Today.svelte          # Today tab — daily summary
│       ├── LogFlow.svelte        # Log to Jira — select + review entries
│       ├── EntryEditor.svelte    # Edit single entry (duration, comment, date)
│       ├── SubmitProgress.svelte # Batch submit progress UI
│       ├── SuccessScreen.svelte  # Post-submit result with logged/kept groups
│       ├── Settings.svelte       # Settings tab
│       └── shared/
│           ├── Checkbox.svelte
│           ├── Badge.svelte
│           ├── DurationInput.svelte
│           └── CommentField.svelte
│
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── icons/                   # App icons (tray + app)
│   │
│   ├── migrations/
│   │   └── 001_init.sql         # Initial SQLite schema
│   │
│   └── src/
│       ├── lib.rs               # Tauri builder: setup tray, register commands, init plugins
│       ├── main.rs              # Entry point (calls lib::run)
│       │
│       ├── commands/
│       │   ├── mod.rs           # Re-exports all command modules
│       │   ├── auth.rs          # jira_login, jira_logout, jira_verify, get_current_user
│       │   ├── tasks.rs         # fetch_my_tasks, search_task, pin_task, unpin_task
│       │   ├── timer.rs         # start_timer, stop_timer, pause_timer, get_active_timer
│       │   ├── worklog.rs       # submit_worklog, submit_batch_worklog
│       │   └── settings.rs      # get_setting, set_setting
│       │
│       ├── jira/
│       │   ├── mod.rs
│       │   ├── client.rs        # JiraClient struct: base_url, auth header, request methods
│       │   └── models.rs        # JiraUser, JiraIssue, JiraWorklog, JiraSearchResult
│       │
│       ├── timer/
│       │   ├── mod.rs
│       │   └── engine.rs        # TimerEngine: state machine, tokio interval, tray updates
│       │
│       └── db/
│           ├── mod.rs
│           └── queries.rs       # All SQLite queries as functions
│
└── docs/
    ├── PRD.docx
    └── mockups/
        ├── main-flow.html
        ├── log-flow.html
        └── partial-log.html
```

---

## Architecture Rules

### Backend (Rust / Tauri)

1. **All Jira API calls go through Rust** — never call Jira from the frontend directly. This keeps credentials out of the webview and avoids CORS.

2. **Credentials live in OS Keychain only** — use the `keyring` crate. Never store tokens in SQLite, localStorage, or config files.

3. **Timer runs in Rust, not JS** — the timer engine uses `tokio::time::interval` to tick every second. It updates the tray title via `tray.set_title()` and emits events to the frontend via `app.emit("timer-tick", payload)`. This ensures accuracy even when the panel window is closed.

4. **Single active timer rule** — the `TimerEngine` enforces that only one timer runs at a time. Starting a new timer automatically stops the current one. This is a state machine with states: `Idle`, `Running(task_id)`, `Paused(task_id)`.

5. **Tauri commands are the API boundary** — every frontend action goes through `#[tauri::command]` functions. Commands are thin wrappers that delegate to `jira::client`, `timer::engine`, or `db::queries`.

6. **Error handling** — all commands return `Result<T, String>`. Map internal errors to user-friendly strings. Never expose raw reqwest or SQLite errors to the frontend.

7. **State management** — use `tauri::Manager` + `app.state::<T>()` for shared state. Wrap mutable state in `Arc<Mutex<T>>` or `Arc<RwLock<T>>`. The `TimerEngine` and `JiraClient` are managed state.

### Frontend (Svelte 5)

1. **Use Svelte 5 Runes** — `$state`, `$derived`, `$effect` for all reactivity. No legacy `$:` reactive declarations or `let` store subscriptions.

2. **Stores are the single source of truth** — `auth.ts`, `tasks.ts`, `timer.ts`, `entries.ts`. Components read from stores, actions dispatch to stores, stores call Tauri commands.

3. **invoke() wrappers in `api/tauri.ts`** — every Tauri command has a typed wrapper function. Components never call `invoke()` directly.

   ```typescript
   // api/tauri.ts
   import { invoke } from '@tauri-apps/api/core';
   import type { JiraUser, Task, TimeEntry } from '$lib/types';

   export const jiraLogin = (domain: string, email: string, token: string) =>
     invoke<JiraUser>('jira_login', { domain, email, token });

   export const fetchMyTasks = () =>
     invoke<Task[]>('fetch_my_tasks');

   export const startTimer = (taskId: string) =>
     invoke<void>('start_timer', { taskId });
   ```

4. **Listen to Tauri events for real-time updates** — timer ticks, tray interactions, and other backend-initiated events use `listen()` from `@tauri-apps/api/event`.

   ```typescript
   import { listen } from '@tauri-apps/api/event';

   // In a component or store $effect:
   const unlisten = await listen<TimerTickPayload>('timer-tick', (event) => {
     elapsed = event.payload.elapsed_secs;
     activeTaskId = event.payload.task_id;
   });
   ```

5. **Dark theme only** — the app uses a fixed dark color scheme. All colors come from CSS custom properties defined in `app.css`. No light mode toggle.

6. **JetBrains Mono for data** — task keys, durations, timestamps, badges use `font-family: var(--font-mono)`. Body text and labels use `var(--font-body)`.

7. **Component size** — keep components under 200 lines. Extract sub-components when a component gets too complex.

8. **No external UI library** — all components are custom-built to match the mockup design. No Tailwind, no component library.

### Database (SQLite)

1. **Migrations are in `migrations/`** — numbered SQL files (e.g., `001_init.sql`). Applied on app startup via `tauri-plugin-sql`.

2. **`time_entries` is the core table** — every timer start creates a row. `end_time` is NULL while running. `synced_to_jira` tracks whether the entry has been logged.

3. **`adjusted_secs` vs `duration_secs`** — `duration_secs` is the raw tracked time. `adjusted_secs` is after rounding or manual editing. Worklog submit uses `adjusted_secs` if set, else `duration_secs`.

4. **Task cache** — `tasks` table caches Jira issue data to reduce API calls. Refresh on manual pull or every 5 minutes when panel is open.

5. **Never delete entries** — use soft flags (`synced_to_jira`, `deleted`) instead of DELETE statements. This preserves history for reporting.

---

## Key Behaviors

### Timer State Machine

```
         start(task_a)
  Idle ─────────────────► Running(task_a)
   ▲                          │
   │ stop()                   │ pause()
   │                          ▼
   │                     Paused(task_a)
   │                          │
   │ stop()                   │ resume()
   │◄─────────────────────────┤
   │                          ▼
   │                     Running(task_a)
   │
   │     start(task_b) while Running(task_a):
   │       1. stop(task_a)  → saves entry with end_time = now
   │       2. start(task_b) → new entry with start_time = now
   │       → Running(task_b)
```

### Tray Title Update Logic

```rust
// Pseudo-code — runs every 1s via tokio interval
match timer_state {
    Idle => {
        let unlogged = db.count_unlogged_today();
        if unlogged > 0 {
            tray.set_title(format!("⏱ CT · {} unlogged", unlogged));
        } else {
            tray.set_title("⏱ CT");
        }
    },
    Running { task_id, start_time, .. } => {
        let elapsed = now() - start_time;
        tray.set_title(format!("{} · {}", task_id, format_duration(elapsed)));
    },
    Paused { task_id, elapsed, .. } => {
        tray.set_title(format!("{} · {} ⏸", task_id, format_duration(elapsed)));
    },
}
```

### Partial Log Flow

1. User opens Today tab → sees all entries for today
2. Clicks "Log to Jira" → enters `LogFlow.svelte`
3. Each entry has a checkbox. Default: stopped timers = checked, running timer = unchecked
4. Quick filters: `All` | `None` | `Stopped` (only stopped timers)
5. Summary bar shows `selected duration / total duration`
6. User can tap an entry to edit (duration, comment, date) via `EntryEditor.svelte`
7. "Log Selected (N)" → `SubmitProgress.svelte` (sequential POST per entry)
8. Result → `SuccessScreen.svelte` with two groups: "✓ Logged" and "⏳ Still Tracking"
9. Logged entries get `synced_to_jira = true`. They appear dimmed in Timer view under "Logged Today"
10. Running timer continues uninterrupted throughout the entire flow

### Worklog Payload

```json
{
  "timeSpentSeconds": 9000,
  "started": "2026-02-20T09:15:00.000+0700",
  "comment": {
    "type": "doc",
    "version": 1,
    "content": [{
      "type": "paragraph",
      "content": [{
        "type": "text",
        "text": "🐛 Bug fix: Fixed HEIF compression pipeline"
      }]
    }]
  }
}
```

---

## SQLite Schema

```sql
CREATE TABLE users (
  id            TEXT PRIMARY KEY,
  email         TEXT NOT NULL,
  display_name  TEXT,
  avatar_url    TEXT,
  jira_domain   TEXT NOT NULL,
  auth_method   TEXT DEFAULT 'api_token',
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE tasks (
  id            TEXT PRIMARY KEY,        -- "ABC-123"
  summary       TEXT NOT NULL,
  project_key   TEXT,
  project_name  TEXT,
  status        TEXT,
  sprint_name   TEXT,
  pinned        BOOLEAN DEFAULT 0,
  last_fetched  DATETIME,
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE time_entries (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id         TEXT NOT NULL REFERENCES tasks(id),
  start_time      DATETIME NOT NULL,
  end_time        DATETIME,              -- NULL while running
  duration_secs   INTEGER,               -- raw tracked time
  adjusted_secs   INTEGER,               -- after rounding/edit
  description     TEXT,                   -- worklog comment
  synced_to_jira  BOOLEAN DEFAULT 0,
  jira_worklog_id TEXT,
  created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE settings (
  key    TEXT PRIMARY KEY,
  value  TEXT
);

CREATE INDEX idx_entries_task ON time_entries(task_id);
CREATE INDEX idx_entries_date ON time_entries(start_time);
CREATE INDEX idx_entries_synced ON time_entries(synced_to_jira);
```

---

## Design Tokens

```css
:root {
  /* Backgrounds */
  --bg-panel: #14171E;
  --bg-card: #1B1F2A;
  --bg-card-hover: #222738;
  --bg-card-active: #1A2435;

  /* Borders */
  --border: #2A2F3D;
  --border-focus: #3D7AED;

  /* Text */
  --text-primary: #E8ECF4;
  --text-secondary: #7A8299;
  --text-muted: #4A5068;

  /* Accents */
  --accent-blue: #3D7AED;
  --accent-green: #2DD4A0;
  --accent-orange: #F0993E;
  --accent-red: #EF5757;
  --accent-purple: #A78BFA;

  /* Typography */
  --font-mono: 'JetBrains Mono', monospace;
  --font-body: 'Outfit', sans-serif;

  /* Radius */
  --radius: 10px;
  --radius-sm: 6px;
}
```

---

## Naming Conventions

| Context | Convention | Example |
|---------|-----------|---------|
| Rust modules | snake_case | `timer_engine`, `jira_client` |
| Rust structs/enums | PascalCase | `TimerEngine`, `TimerState`, `JiraUser` |
| Rust functions | snake_case | `start_timer()`, `fetch_my_tasks()` |
| Tauri commands | snake_case | `#[tauri::command] fn start_timer()` |
| Svelte components | PascalCase | `TaskCard.svelte`, `LogFlow.svelte` |
| Svelte store files | camelCase | `auth.ts`, `timer.ts` |
| TS functions | camelCase | `formatDuration()`, `roundToNearest()` |
| TS types/interfaces | PascalCase | `TimeEntry`, `JiraUser`, `TimerState` |
| CSS variables | kebab-case | `--bg-panel`, `--accent-blue` |
| SQL tables | snake_case | `time_entries` |
| SQL columns | snake_case | `start_time`, `synced_to_jira` |
| DB migrations | numbered prefix | `001_init.sql`, `002_add_settings.sql` |
| Events (Tauri) | kebab-case | `timer-tick`, `timer-stopped` |

---

## Jira API Reference

### Auth Header (API Token — MVP)
```
Authorization: Basic base64("{email}:{api_token}")
Content-Type: application/json
```

### Endpoints Used

| Command | Endpoint | Method | Phase |
|---------|----------|--------|-------|
| Verify auth | `/rest/api/3/myself` | GET | MVP |
| Fetch tasks | `/rest/api/3/search?jql={jql}&maxResults=50` | GET | MVP |
| Get issue | `/rest/api/3/issue/{key}` | GET | MVP |
| Add worklog | `/rest/api/3/issue/{key}/worklog` | POST | MVP |

### Default JQL
```
assignee = currentUser()
AND status IN ("To Do", "In Progress")
AND sprint IN openSprints()
ORDER BY updated DESC
```

### Error Handling

| HTTP Status | Meaning | App Behavior |
|-------------|---------|-------------|
| 401 | Auth invalid/expired | Prompt re-login, clear keychain |
| 403 | No permission | Error badge on entry, skip |
| 404 | Issue not found | Error, allow retry or remove |
| 429 | Rate limited | Retry with backoff (1s, 2s, 4s) |
| 5xx | Server error | Retry up to 3 times, then error |

---

## Common Commands

```bash
# ── Development ──
npm install                      # Install frontend deps
npm run tauri dev                # Run in dev mode (hot reload)
npm run tauri build              # Production build

# ── Rust ──
cd src-tauri
cargo test                       # Run tests
cargo clippy                     # Lint
cargo fmt                        # Format

# ── Database (debug) ──
# macOS:  ~/Library/Application Support/id.rickyirfandi.catettask/catet-task.db
# Linux:  ~/.local/share/id.rickyirfandi.catettask/catet-task.db
# Windows: %APPDATA%/id.rickyirfandi.catettask/catet-task.db
sqlite3 <path>/catet-task.db ".tables"
sqlite3 <path>/catet-task.db "SELECT * FROM time_entries WHERE date(start_time) = date('now');"
```

---

## Implementation Notes

### Tray Title Performance
- `tray.set_title()` is called every 1 second — this is lightweight on all platforms.
- On macOS, use a **template image** for the tray icon so it adapts to light/dark menu bar.
- The Unicode dot `●` renders correctly cross-platform in tray titles.

### Panel Window
- Size: `340 × 520px`, frameless (`decorations: false`)
- Position: anchored below tray icon via `tauri-plugin-positioner` (`Position::TrayBottomCenter`)
- macOS: use `tauri-nspanel` for native popover behavior (auto-dismiss on click outside)
- Hidden from Dock: set `LSUIElement = true` in `Info.plist`
- Panel should NOT steal focus from the user's current app

### Offline Resilience
- All timer data stored locally in SQLite immediately
- App works fully offline for time tracking
- Only task fetch and worklog submit need connectivity
- If Jira is unreachable during submit, entries stay local with `synced_to_jira = false`

### Duration Rounding
```typescript
function roundToNearest(seconds: number, minuteIncrement: number): number {
  const totalMinutes = seconds / 60;
  const rounded = Math.round(totalMinutes / minuteIncrement) * minuteIncrement;
  return Math.max(minuteIncrement, rounded) * 60; // minimum 1 increment
}
```

### Keyboard Shortcuts (Global)
| Shortcut | Action |
|----------|--------|
| `CmdOrCtrl+Shift+T` | Toggle panel |
| `CmdOrCtrl+Shift+P` | Pause / Resume |
| `CmdOrCtrl+Shift+L` | Open log flow |
| `CmdOrCtrl+K` | Focus search (panel open) |
| `Escape` | Close panel |

---

## Testing Checklist

### Timer
- [ ] Start timer → tray shows `TASK-ID · 00:00:01` incrementing
- [ ] Switch task → previous timer stops with correct duration, new one starts
- [ ] Pause → tray shows `⏸`, time freezes
- [ ] Resume → time continues from paused point
- [ ] Close panel → timer continues in Rust background
- [ ] Reopen panel → shows correct elapsed time from event listener

### Partial Log
- [ ] Running task is unchecked by default
- [ ] "Stopped" filter → only shows stopped entries
- [ ] Select 2 of 4 → summary shows `selected / total`
- [ ] Submit selected → only those entries get `synced_to_jira = true`
- [ ] Running timer uninterrupted during and after submit
- [ ] Logged entries appear dimmed under "Logged Today"
- [ ] Unlogged entries remain available for next log session

### Auth
- [ ] Valid credentials → login success, user avatar + name shown
- [ ] Invalid credentials → error message, can retry
- [ ] App restart → auto-login from Keychain silently
- [ ] Logout → Keychain cleared, back to login screen
- [ ] Token expired (401 on API call) → prompt re-login

### Edge Cases
- [ ] No internet → timer works, fetch/submit show offline error
- [ ] Jira returns 429 → auto-retry with exponential backoff
- [ ] Task deleted in Jira → error on submit, allow entry removal
- [ ] Midnight rollover → entries correctly attributed to the right day
- [ ] 8h+ durations → display and submit correctly
- [ ] Rapid task switching (< 1 second) → no lost or duplicate entries
- [ ] Submit with empty comment → allowed (comment is optional in Jira)
- [ ] Submit with 0 duration → blocked (minimum 1 rounding increment)
