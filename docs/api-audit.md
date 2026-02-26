# Jira API Security Audit

**Date**: 2026-02-21
**Scope**: All outbound HTTP calls from the app to Jira Cloud REST API

---

## Summary

The app only **READs** tasks and **WRITEs worklogs**. It cannot edit, delete, transition, or modify anything else in Jira.

- No `PUT` calls (no editing issues/tasks)
- No `DELETE` calls (no deleting anything)
- No transition endpoints (`/transitions`)
- No issue edit endpoints (`PUT /issue/{key}`)
- No comment endpoints (only worklog)
- No assignee/label/priority modification
- No project/board/sprint modification

---

## All Jira HTTP Calls

Source: `src-tauri/src/jira/client.rs`

| # | Method | Endpoint | Purpose | Verdict |
|---|--------|----------|---------|---------|
| 1 | `GET` | `/rest/api/3/myself` | Verify auth / get user info | READ only |
| 2 | `POST` | `/rest/api/3/search/jql` | Search issues by JQL | READ only (search is read) |
| 3 | `GET` | `/rest/api/3/issue/{key}` | Get single issue details | READ only |
| 4 | `POST` | `/rest/api/3/issue/{key}/worklog` | Add worklog entry | WRITE — worklog only |

---

## All Tauri Commands

Source: `src-tauri/src/lib.rs`

| Command | Touches Jira? | What it does |
|---------|--------------|-------------|
| `jira_login` | GET `/myself` | Verify credentials |
| `jira_logout` | No | Clear local keychain |
| `jira_verify` | GET `/myself` | Auto-login check |
| `get_current_user` | No | Read local state |
| `fetch_my_tasks` | POST `/search/jql` | Search tasks (read) |
| `search_task` | POST `/search/jql` | Search tasks (read) |
| `pin_task` | No | Local SQLite only |
| `unpin_task` | No | Local SQLite only |
| `start_timer` | No | Local timer + SQLite |
| `stop_timer` | No | Local timer + SQLite |
| `pause_timer` | No | Local timer only |
| `resume_timer` | No | Local timer only |
| `get_active_timer` | No | Read local state |
| `get_entries_today` | No | Read local SQLite |
| `update_entry` | No | Local SQLite only |
| `submit_batch_worklog` | POST `/worklog` | Only write — adds worklog |
| `get_setting` | No | Local SQLite only |
| `set_setting` | No | Local SQLite only |

---

## Frontend Security

- All Jira calls go through Rust backend (no direct `fetch()` to Jira from the webview)
- Credentials never exposed to frontend (stored in OS Keychain, used only in Rust)
- No raw `invoke()` calls in components — all go through typed wrappers in `src/lib/api/tauri.ts`

---

## Credential Storage

- API token stored in OS Keychain via `keyring` crate (service: `catet-task`)
- Never stored in SQLite, localStorage, or config files
- Auth header constructed in Rust memory only (`Basic base64(email:token)`)
- Cleared from keychain on logout
