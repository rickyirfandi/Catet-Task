


**Catet Task**

Product Requirements Document

Menu Bar Time Tracking App with Jira Integration

Version 1.0  •  February 20, 2026

Author: Ricky Wijaya


# **Table of Contents**
1\. Overview

2\. Problem Statement

3\. Target User

4\. Tech Stack

5\. Architecture

6\. Authentication

7\. Core Features (MVP)

8\. UI/UX Specification

9\. Data Model

10\. Jira API Integration

11\. Phase 2 Features

12\. Development Plan

13\. Success Metrics


# **1. Overview**
Catet Task is a lightweight menu bar application that lives in the system tray and provides seamless time tracking for Jira tasks. Users can start/stop timers per task, switch between tasks with automatic timer management, and batch-submit worklogs to Jira at the end of the day — all without leaving their workflow.

The app is built with Rust (Tauri v2) for the backend and Svelte 5 for the frontend, ensuring a native-feeling experience with minimal resource usage. The system tray shows the currently active task and elapsed time in real-time (e.g., "ABC-123 · 00:34:12").

|**Attribute**|**Detail**|
| :- | :- |
|**App Name**|Catet Task|
|**Type**|System Tray / Menu Bar Application|
|**Platforms**|macOS (primary), Windows, Linux|
|**Backend**|Rust (Tauri v2)|
|**Frontend**|Svelte 5 (Runes)|
|**Database**|SQLite (local)|
|**Jira Support**|Jira Cloud + Jira Server/Data Center|

# **2. Problem Statement**
Developers and team members who use Jira often struggle with accurate time logging. Common pain points include:

- Forgetting to log time at the end of the day, resulting in inaccurate worklogs.
- Context switching between tasks without tracking how much time was actually spent on each.
- The friction of opening Jira in a browser, navigating to the issue, and manually entering worklog data.
- Guessing time allocation after the fact, leading to unreliable project metrics.

Catet Task solves this by making time tracking effortless — always visible in the menu bar, one-click task switching, and batch worklog submission with comment support.
# **3. Target User**
The primary user is a developer or team member who uses Jira daily and needs to log time per task. They value speed, minimal disruption, and accuracy.

|**Persona**|**Description**|**Key Need**|
| :- | :- | :- |
|**Developer**|Works on 3-5 Jira tasks per day, switches context frequently|Auto-switch timers, batch log|
|**Tech Lead**|Reviews PRs, attends meetings, does code work|Track non-coding time too|
|**QA Engineer**|Tests multiple issues per sprint|Quick task search, fast logging|
|**Project Manager**|Needs accurate time data for reporting|Reliable worklog data in Jira|

# **4. Tech Stack**

|**Layer**|**Technology**|**Rationale**|
| :- | :- | :- |
|**App Shell**|Tauri v2 (Rust)|Native performance, small binary (~5MB), system tray support|
|**Frontend**|Svelte 5 (Runes)|Reactive UI, minimal bundle, fast rendering|
|**State Mgmt**|Svelte Stores|Simple, reactive, no external dependency|
|**Local DB**|SQLite (tauri-plugin-sql)|Persist time entries, offline support|
|**HTTP Client**|reqwest (Rust)|Async HTTP for Jira API calls|
|**Credential Store**|keyring crate|OS Keychain (macOS Keychain, Windows Credential Vault)|
|**Tray**|tauri tray-icon|Native system tray with dynamic title|
|**Positioning**|tauri-plugin-positioner|Anchor popup window near tray icon|
|**macOS Panel**|tauri-nspanel (optional)|Native NSPanel behavior (click-outside dismiss)|

# **5. Architecture**
## **5.1 High-Level Architecture**
The application follows a clean separation between the Rust backend (Tauri commands) and the Svelte frontend. All Jira API communication happens through the Rust backend to keep credentials secure and avoid CORS issues.

┌─────────────────────────────────────────────────┐\
│                  System Tray                     │\
│         [● ABC-123 · 00:34:12]                  │\
└──────────────────┬──────────────────────────────┘\
`                   `│ click\
┌──────────────────▼──────────────────────────────┐\
│            Svelte Frontend (Panel)               │\
│  ┌─────────┐  ┌─────────┐  ┌──────────────┐    │\
│  │  Timer   │  │  Today  │  │   Settings   │    │\
│  │  View    │  │  View   │  │    View      │    │\
│  └─────────┘  └─────────┘  └──────────────┘    │\
└──────────────────┬──────────────────────────────┘\
`                   `│ invoke()\
┌──────────────────▼──────────────────────────────┐\
│           Rust Backend (Tauri Commands)          │\
│  ┌───────────┐  ┌───────────┐  ┌────────────┐  │\
│  │ Jira API  │  │  Timer    │  │  Storage   │  │\
│  │ Client    │  │  Engine   │  │  Manager   │  │\
│  └─────┬─────┘  └───────────┘  └──────┬─────┘  │\
└────────┼────────────────────────────────┼───────┘\
`         `│                                │\
`    `┌────▼────┐                    ┌──────▼──────┐\
`    `│  Jira   │                    │   SQLite    │\
`    `│  Cloud  │                    │   + Keyring │\
`    `└─────────┘                    └─────────────┘
## **5.2 Tauri Command Modules**

|**Module**|**Responsibility**|**Key Commands**|
| :- | :- | :- |
|**jira\_auth**|Authentication & credential management|jira\_login, jira\_logout, jira\_verify|
|**jira\_tasks**|Fetch & search Jira issues|fetch\_my\_tasks, search\_task, get\_task\_detail|
|**jira\_worklog**|Submit worklogs to Jira|submit\_worklog, submit\_batch\_worklog|
|**timer**|Timer lifecycle management|start\_timer, stop\_timer, pause\_timer, get\_active|
|**storage**|Local DB operations|get\_entries\_today, update\_entry, delete\_entry|
|**tray**|System tray updates|update\_tray\_title, update\_tray\_icon|

# **6. Authentication**
## **6.1 Supported Auth Methods**

|**Method**|**Target**|**How It Works**|**Phase**|
| :- | :- | :- | :- |
|**API Token**|Jira Cloud|User provides email + API token. Stored in OS Keychain. Sent as Basic Auth header.|MVP|
|**OAuth 2.0 (3LO)**|Jira Cloud|OAuth flow via browser redirect. Auto-refresh tokens. Best for distribution.|Phase 2|
|**Personal Access Token**|Jira Server/DC|Bearer token for self-hosted Jira instances.|Phase 2|

## **6.2 MVP Auth Flow (API Token)**
1. User opens Catet Task for the first time → Login screen appears.
1. User inputs: Jira Domain, Email, API Token.
1. App calls GET /rest/api/3/myself to verify credentials.
1. On success: credentials stored in OS Keychain (via keyring crate), user info cached in SQLite.
1. On failure: show error message, allow retry.
1. Subsequent launches: auto-load credentials from Keychain, verify silently.
## **6.3 Credential Security**
- API tokens are NEVER stored in plain text or in SQLite.
- OS Keychain provides hardware-backed encryption on macOS (Secure Enclave).
- Credentials are identified by a service name ("catet-task") and account (user email).
- Logout action deletes credentials from Keychain and clears cached user data.
# **7. Core Features (MVP)**
## **7.1 System Tray Integration**
The system tray is the primary interface. It shows the app state at a glance without opening any window.

|**State**|**Tray Display**|**Icon**|
| :- | :- | :- |
|**Idle (no timer)**|⏱ Catet Task|Default icon, gray dot|
|**Timer running**|ABC-123 · 00:34:12|Green pulsing dot|
|**Timer paused**|ABC-123 · 00:34:12 ⏸|Orange blinking dot|
|**Unsynced entries**|⏱ Catet Task · 3 unlogged|Default icon with badge|

The tray title updates every second when a timer is running. This is achieved by a Rust-side interval that calls tray.set\_title() with the formatted string. Clicking the tray icon toggles the panel window positioned directly below the icon.
## **7.2 Task Management**
- **Fetch —** Fetch Assigned Tasks: 

On app launch and on manual refresh, fetch tasks assigned to the current user using JQL: assignee = currentUser() AND status IN ("To Do", "In Progress") AND sprint IN openSprints() ORDER BY updated DESC. Cache results in SQLite with a last\_fetched timestamp.

- **Search —** Search Tasks: 

Users can search by task key (e.g., "ABC-123") or by text in the summary. The search bar supports both JQL key lookup and text search against the local cache, with fallback to Jira API for uncached results.

- **Pin —** Pin Favorites: 

Frequently used tasks can be pinned to the top of the task list. Pinned tasks persist across sessions and are always visible regardless of sprint or status filters.
## **7.3 Timer Engine**
The timer engine enforces a single-active-timer rule. Core behaviors:

|**Action**|**Behavior**|**Side Effects**|
| :- | :- | :- |
|**Play Task A**|Start timer for A|Create time\_entry with start\_time = now|
|**Play Task B (while A running)**|Stop A, Start B|Set A.end\_time = now, Create B entry|
|**Pause active task**|Pause current timer|Store elapsed, tray shows ⏸|
|**Resume paused task**|Continue timer|Adjust start\_time to account for pause|
|**Stop active task**|Stop timer|Set end\_time, calculate duration|

The timer runs in the Rust backend using tokio intervals, not in the frontend. This ensures accuracy even if the panel window is closed. The frontend receives timer updates via Tauri event system (emit/listen).
## **7.4 Daily Summary & Partial Logging**
The "Today" tab shows all time entries for the current day. Users can review, edit, and selectively submit entries to Jira.
### **7.4.1 Partial Log (Selective Submit)**
Users can choose which entries to log. Not all tasks need to be logged at once.

- Each entry has a checkbox for selection.
- Quick filter buttons: "All" / "None" / "Stopped only" (exclude running timers).
- Running tasks show a green "● running" badge and are deselected by default.
- Summary bar dynamically shows: selected duration / total duration (e.g., "4h 30m / 6h 22m").
- Footer button shows count: "🚀 Log Selected (2)".
- A yellow callout warns if a running task is being skipped.
### **7.4.2 Entry Editing (Pre-Submit)**
Before submitting, users can tap any entry to edit details:

- **Duration —** Duration: 

Editable with hour:minute segments, quick presets (15m, 30m, 1h, 2h, 4h), and up/down arrows. Auto-rounding to configurable increments (1m, 15m, 30m) with original time shown as strikethrough.

- **Date —** Date: 

Defaults to today. Can be changed for backdated logging.

- **Time Range —** Time Range: 

Shows actual start → end times. Read-only reference.

- **Work Description —** Work Description: 

Free text that becomes the Jira worklog comment. Quick-insert templates: 🐛 Bug fix, ✨ Feature, 🔧 Refactor, 📝 Review, 🔍 Investigation. Templates prepend the emoji prefix and a colon to the description.
### **7.4.3 Submit Flow**
1. User clicks "Log Selected (N)" button.
1. App sends POST /rest/api/3/issue/{key}/worklog for each selected entry sequentially.
1. Progress UI shows per-entry status: ✓ done, ● loading, ○ pending.
1. On success: entry marked as synced\_to\_jira = true, jira\_worklog\_id stored.
1. On partial failure: failed entries show error badge + retry link. Successful entries remain synced.
1. Success screen shows two groups: "✓ Logged to Jira" and "⏳ Still Tracking".
### **7.4.4 Post-Log Behavior**
After a partial log, the Timer view reorganizes into three sections:

- **Currently Tracking —** Currently Tracking: 

Active timer, uninterrupted by the log action.

- **Unlogged —** Unlogged: 

Skipped/unselected entries. Ready for the next log session.

- **Logged Today —** Logged Today: 

Dimmed/collapsed entries that have been synced. Won't appear in the next log session.
# **8. UI/UX Specification**
## **8.1 Panel Window**

|**Property**|**Value**|
| :- | :- |
|**Size**|340 × 520px (adjustable based on content)|
|**Position**|Anchored below tray icon (via tauri-plugin-positioner)|
|**Decorations**|None (frameless, custom title bar)|
|**Style**|Dark theme, rounded corners, subtle shadow|
|**Behavior**|Click outside to dismiss (NSPanel on macOS)|
|**Dock**|Hidden from dock (LSUIElement = true on macOS)|

## **8.2 Navigation**
The panel has three tabs in a segmented control at the top:

|**Tab**|**Purpose**|**Key Elements**|
| :- | :- | :- |
|**Timer**|Task list + active timer|Search bar, task cards with play/pause, grouped sections|
|**Today**|Daily summary + log to Jira|Entry list with checkboxes, duration bar, submit button|
|**Settings (⚙)**|Configuration|Auth settings, rounding preference, notification toggle|

## **8.3 Design Tokens**

|**Token**|**Value**|**Usage**|
| :- | :- | :- |
|**--bg-panel**|#14171E|Panel background|
|**--bg-card**|#1B1F2A|Card/input background|
|**--border**|#2A2F3D|Borders and dividers|
|**--text-primary**|#E8ECF4|Primary text|
|**--text-secondary**|#7A8299|Secondary/label text|
|**--accent-blue**|#3D7AED|Links, active states, task keys|
|**--accent-green**|#2DD4A0|Running timer, success states|
|**--accent-orange**|#F0993E|Paused timer, warnings|
|**--accent-red**|#EF5757|Errors, delete actions|
|**--font-mono**|JetBrains Mono|Task keys, durations, code-like text|
|**--font-body**|Outfit|Body text, labels, buttons|

## **8.4 Keyboard Shortcuts**

|**Shortcut**|**Action**|
| :- | :- |
|**⌘ + Shift + T**|Toggle panel visibility|
|**⌘ + Shift + P**|Pause / Resume active timer|
|**⌘ + Shift + L**|Open "Log to Jira" flow|
|**⌘ + K**|Focus task search bar|
|**Escape**|Close panel|

# **9. Data Model**
## **9.1 SQLite Schema**
-- User & auth metadata\
CREATE TABLE users (\
`  `id            TEXT PRIMARY KEY,\
`  `email         TEXT NOT NULL,\
`  `display\_name  TEXT,\
`  `avatar\_url    TEXT,\
`  `jira\_domain   TEXT NOT NULL,\
`  `auth\_method   TEXT DEFAULT 'api\_token',\
`  `created\_at    DATETIME DEFAULT CURRENT\_TIMESTAMP\
);\
\
-- Cached Jira tasks\
CREATE TABLE tasks (\
`  `id            TEXT PRIMARY KEY,        -- e.g. "ABC-123"\
`  `summary       TEXT NOT NULL,\
`  `project\_key   TEXT,\
`  `project\_name  TEXT,\
`  `status        TEXT,\
`  `sprint\_name   TEXT,\
`  `pinned        BOOLEAN DEFAULT 0,\
`  `last\_fetched  DATETIME,\
`  `created\_at    DATETIME DEFAULT CURRENT\_TIMESTAMP\
);\
\
-- Time tracking entries\
CREATE TABLE time\_entries (\
`  `id              INTEGER PRIMARY KEY AUTOINCREMENT,\
`  `task\_id         TEXT NOT NULL REFERENCES tasks(id),\
`  `start\_time      DATETIME NOT NULL,\
`  `end\_time        DATETIME,\
`  `duration\_secs   INTEGER,               -- calculated on stop\
`  `adjusted\_secs   INTEGER,               -- after rounding/manual edit\
`  `description     TEXT,                   -- worklog comment\
`  `synced\_to\_jira  BOOLEAN DEFAULT 0,\
`  `jira\_worklog\_id TEXT,                   -- returned after submit\
`  `created\_at      DATETIME DEFAULT CURRENT\_TIMESTAMP\
);\
\
-- App settings\
CREATE TABLE settings (\
`  `key    TEXT PRIMARY KEY,\
`  `value  TEXT\
);\
\
-- Indexes\
CREATE INDEX idx\_entries\_task ON time\_entries(task\_id);\
CREATE INDEX idx\_entries\_date ON time\_entries(start\_time);\
CREATE INDEX idx\_entries\_synced ON time\_entries(synced\_to\_jira);
## **9.2 Settings Keys**

|**Key**|**Default**|**Description**|
| :- | :- | :- |
|**round\_to\_minutes**|15|Round durations to nearest N minutes (1, 15, 30)|
|**auto\_round**|true|Automatically round on log screen|
|**idle\_detection**|true|Detect AFK and prompt user|
|**idle\_threshold\_minutes**|5|Minutes before idle prompt|
|**reminder\_enabled**|true|End-of-day log reminder|
|**reminder\_time**|17:00|Time for daily reminder notification|
|**default\_jql**|assignee = currentUser()...|JQL query for task fetching|

# **10. Jira API Integration**
## **10.1 API Endpoints**

|**Endpoint**|**Method**|**Purpose**|**Phase**|
| :- | :- | :- | :- |
|**/rest/api/3/myself**|GET|Verify auth, get user info|MVP|
|**/rest/api/3/search?jql=...**|GET|Fetch assigned tasks by JQL|MVP|
|**/rest/api/3/issue/{key}**|GET|Get single issue detail|MVP|
|**/rest/api/3/issue/{key}/worklog**|POST|Submit time log entry|MVP|
|**/rest/api/3/issue/{key}/transitions**|GET|Get available status transitions|Phase 2|
|**/rest/api/3/issue/{key}/transitions**|POST|Transition issue status|Phase 2|

## **10.2 Worklog Payload**
POST /rest/api/3/issue/ABC-123/worklog\
Authorization: Basic base64(email:token)\
Content-Type: application/json\
\
{\
`  `"timeSpentSeconds": 9000,          // 2h 30m\
`  `"started": "2026-02-20T09:15:00.000+0700",\
`  `"comment": {\
`    `"type": "doc",\
`    `"version": 1,\
`    `"content": [{\
`      `"type": "paragraph",\
`      `"content": [{\
`        `"type": "text",\
`        `"text": "🐛 Bug fix: Fixed HEIF compression pipeline"\
`      `}]\
`    `}]\
`  `}\
}
## **10.3 Error Handling**

|**HTTP Status**|**Meaning**|**App Behavior**|
| :- | :- | :- |
|**401**|Auth expired/invalid|Prompt re-login, clear keychain entry|
|**403**|No permission to log|Show error badge on entry, skip|
|**404**|Issue not found/deleted|Show error, allow retry or remove|
|**429**|Rate limited|Retry with exponential backoff (1s, 2s, 4s)|
|**5xx**|Jira server error|Retry up to 3 times, then show error|

# **11. Phase 2 Features**
These features are planned for after MVP launch, prioritized by user value:

|**Feature**|**Description**|**Priority**|
| :- | :- | :- |
|**Idle Detection**|Detect AFK after configurable threshold. Popup: "You were idle since 14:32. Keep or discard?"|High|
|**OAuth 2.0**|Full OAuth flow for Jira Cloud. Auto-refresh tokens.|High|
|**Jira Server PAT**|Support for self-hosted Jira (Data Center) via Personal Access Token.|Medium|
|**Daily Reminder**|System notification at configurable time (e.g., 5 PM) if unlogged entries exist.|Medium|
|**Pomodoro Mode**|Optional 25/5 focus timer overlay per task.|Low|
|**Weekly Report**|Summary view: time per project per day, exportable as CSV/image.|Medium|
|**Offline Mode**|Queue worklogs locally when offline, auto-sync when connection restored.|High|
|**Status Transition**|Option to auto-move Jira issue to "In Progress" when timer starts.|Low|
|**Comment Templates**|Saveable custom comment templates beyond the default emoji presets.|Low|
|**Multi-Account**|Support multiple Jira instances (e.g., work + freelance).|Low|

# **12. Development Plan**
## **12.1 MVP Milestones**

|**Week**|**Milestone**|**Deliverables**|
| :- | :- | :- |
|**Week 1**|Project Setup & Shell|Tauri + Svelte scaffold, system tray with static icon, panel window with positioning, dark theme foundation|
|**Week 2**|Auth & Jira Connection|Login screen UI, API token auth flow, OS Keychain integration, /myself verification, user info display|
|**Week 3**|Task Fetching & Display|JQL task fetch, SQLite caching, task list UI with search, pin/favorite functionality|
|**Week 4**|Timer Engine|Rust-side timer with tokio, start/stop/pause/switch logic, real-time tray title updates, frontend timer display|
|**Week 5**|Daily Summary & Partial Log|Today view, entry list with checkboxes, selective submit, duration editing, rounding|
|**Week 6**|Worklog Submit & Polish|Comment editing with templates, batch submit with progress UI, error handling, post-log state management|
|**Week 7**|Testing & Release|End-to-end testing, edge cases, macOS notarization, build pipeline, v0.1.0 release|

## **12.2 Project Structure**
catet-task/\
├── src-tauri/\
│   ├── Cargo.toml\
│   ├── tauri.conf.json\
│   ├── icons/\
│   ├── migrations/           # SQLite migrations\
│   │   └── 001\_init.sql\
│   └── src/\
│       ├── lib.rs            # Tauri setup, tray, window\
│       ├── commands/\
│       │   ├── mod.rs\
│       │   ├── auth.rs       # jira\_login, jira\_logout\
│       │   ├── tasks.rs      # fetch\_tasks, search\_task\
│       │   ├── timer.rs      # start, stop, pause, switch\
│       │   ├── worklog.rs    # submit\_worklog, batch\
│       │   └── settings.rs   # get/set settings\
│       ├── jira/\
│       │   ├── mod.rs\
│       │   ├── client.rs     # reqwest HTTP client\
│       │   └── models.rs     # Jira API types\
│       ├── timer/\
│       │   ├── mod.rs\
│       │   └── engine.rs     # Timer state machine\
│       └── db/\
│           ├── mod.rs\
│           └── queries.rs    # SQLite queries\
├── src/\
│   ├── App.svelte\
│   ├── main.ts\
│   ├── lib/\
│   │   ├── stores/\
│   │   │   ├── auth.ts       # Auth state\
│   │   │   ├── tasks.ts      # Task list state\
│   │   │   ├── timer.ts      # Active timer state\
│   │   │   └── entries.ts    # Today's entries\
│   │   ├── api/\
│   │   │   └── tauri.ts      # invoke() wrappers\
│   │   └── utils/\
│   │       ├── time.ts       # Duration formatting\
│   │       └── rounding.ts   # Duration rounding logic\
│   ├── components/\
│   │   ├── Login.svelte\
│   │   ├── Timer.svelte\
│   │   ├── TaskCard.svelte\
│   │   ├── Today.svelte\
│   │   ├── LogFlow.svelte\
│   │   ├── EntryEditor.svelte\
│   │   ├── SubmitProgress.svelte\
│   │   ├── Settings.svelte\
│   │   └── shared/\
│   │       ├── SearchBar.svelte\
│   │       ├── Checkbox.svelte\
│   │       └── Badge.svelte\
│   └── styles/\
│       └── global.css        # Design tokens, base styles\
├── package.json\
├── svelte.config.js\
├── vite.config.ts\
└── README.md
# **13. Success Metrics**

|**Metric**|**Target**|**How to Measure**|
| :- | :- | :- |
|**Daily Active Usage**|Use Catet Task every workday|Local analytics: app open events|
|**Log Accuracy**|> 90% of work hours logged|Compare tracked time vs expected hours|
|**Time to Log**|< 30 seconds for full day|Measure from "Log to Jira" click to success|
|**Timer Coverage**|> 80% of work time tracked|Total tracked / total work hours|
|**Memory Usage**|< 50MB RSS|Activity Monitor / Task Manager|
|**Binary Size**|< 15MB|Build output size|
|**Startup Time**|< 1 second|Time from launch to tray icon visible|


*— End of Document —*
