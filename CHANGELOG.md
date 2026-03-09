# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-03-09

Changes in this release are based on commits between tags `v0.1.0..v0.2.1`.

### Added

- `catet-cli` companion with MCP server support for Claude Desktop integration.
- Tauri sidecar packaging flow for `catet-cli` and app-level Claude connect/disconnect controls.
- Cross-user Jira task search with sprint prioritization.
- Project-scoped search and parent story metadata support.
- Project filter chips in task search.
- Entry editor reintegrated into LogFlow with larger edit affordance.
- Start time editing support in entries UI.
- Submission dry-run flow via `catet_submit_preview`.
- Forgotten timer recovery MCP tools: `catet_plan_manual_log` and `catet_add_manual_log`.
- Versioned migration system with transactional safety.

### Changed

- Search behavior now combines local cache and Jira remote lookup more robustly.
- Task search and manual-log flows improved to cache remote Jira hits for follow-up actions.
- Input and runtime validation hardened for duration handling and binary usability checks.
- App/CLI packaging and integration docs/assets updated.

### Fixed

- Race condition in entry editor save flow.
- Stale project filter behavior after task/search updates.
- P0 issues in retry handling, zero-duration validation, and date updates.
- Multiple UX regressions around empty states, search clear behavior, logged styling, settings feedback, pinned tasks, and login errors.
- Bugs identified in CLI review and follow-up robustness pass.

## [0.1.0] - 2026-03-05

### Added

- Weekly tab with the same reporting model as Today (without Jira submit flow).
- Task detail metadata section: issue type, priority, assignee, created/updated, description.
- Local-timezone rendering for session times in detail view.
- Local-timezone based date grouping for Today/Weekly queries.
- Daily reminder time input with local timezone context.
- Export improvements for report-friendly clipboard and CSV output.
- Open Jira action from success screen.
- GitHub Actions workflow for Windows builds.
- macOS CI matrix support for Apple Silicon and Intel targets.

### Changed

- App version bumped to `0.1.0`.
- Reminder scheduler made more robust with:
  - non-exact-minute trigger logic,
  - persisted last-fired date guard,
  - safer periodic checks.
- Reminder UI updated for clearer local-time interpretation.
- Login defaults now prefill Jira domain with `lgq-team.atlassian.net`.

### Fixed

- Timer robustness across sleep/wake and force-close restart recovery.
- Tray quit path reliability from Settings and tray menu actions.
- Success screen duration/rounding display consistency.
- Success icon rendering (replaced stretched glyph rendering with stable SVG icon).
- Search state consistency when switching tabs (input and filtered list no longer desync).

### Notes

- Build behavior in this workspace may still depend on local environment permissions
  (for example `esbuild spawn EPERM`).
