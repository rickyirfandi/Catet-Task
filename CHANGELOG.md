# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
