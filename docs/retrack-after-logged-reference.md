# Retrack After Logged - Comprehensive Design Reference

## 1. Document Purpose

This document is a technical reference for enabling users to track time again on tasks that were already logged to Jira on the same day.

It captures:

- Current behavior and root cause
- Why a naive UI-only fix is risky
- Proposed phased implementation (safe separation of scope)
- Expected impact and risk analysis
- Edge-case handling
- QA and rollout checklist

This is intended as implementation guidance for later execution.

---

## 2. Problem Statement

### 2.1 User-facing issue

After a task is logged to Jira, users currently cannot start tracking that task again from the Timer tab.

### 2.2 Why this is operationally painful

- Real workflows often require multiple work intervals on the same issue in one day.
- Users may log mid-day, continue work, then need to log again later.
- Current behavior forces awkward workarounds and risks inaccurate logs.

### 2.3 Must-have outcome

- Users can log a task and later track it again on the same day.
- Subsequent log submissions must include only new unsynced sessions.
- No duplicate worklog submission caused by aggregation mistakes.

---

## 3. Current Behavior (As-Is)

### 3.1 UI grouping logic prevents retracking

In Timer view, tasks are split into:

- `Unlogged`
- `My Tasks`
- `Logged Today`

Current filtering excludes any task id that appears in logged entries from the actionable groups, so it only appears in `Logged Today` (rendered non-interactive).

### 3.2 Logged cards are intentionally non-clickable

`TaskCard` has a `logged` mode where clicks return early and no tracking action happens.

### 3.3 Hidden correctness risk in LogFlow aggregation

Current aggregation is task-level and marks aggregate `isSynced = true` if **any** entry for that task is synced.

Consequence:

- If task `ABC-123` has old synced entries + new unsynced entries, aggregate can be treated as synced and excluded from LogFlow.
- This can block valid second submission or create mismatch between shown duration and submitted entry ids.

This means a simple "make logged card clickable" change is not enough.

---

## 4. Root Cause Summary

There are two coupled concerns currently mixed in one aggregation path:

1. **Display concern**: show today totals and task rows.
2. **Submission concern**: choose unsynced entries only for Jira submit.

Because both concerns share one aggregate shape (`isSynced` at task level), partial-sync states are represented incorrectly for the submission workflow.

---

## 5. Design Goals and Non-Goals

## 5.1 Goals

1. Allow retracking of previously logged tasks.
2. Preserve current timer behavior (start/pause/resume/stop robustness).
3. Ensure second/third submission only includes unsynced entries.
4. Keep changes isolated and low-risk via phased rollout.

## 5.2 Non-Goals

1. No schema migration for `time_entries`.
2. No Jira API contract changes.
3. No redesign of all screens; only targeted behavior changes.

---

## 6. Proposed Strategy (Safest Path)

Implement in two separate scopes:

## Scope A - Data correctness first (mandatory)

Create a separate unlogged-only aggregation path for LogFlow and related submission logic.

If Scope A is not done, Scope B can create incorrect/dangerous submission behavior.

## Scope B - UX enable retracking

After Scope A is stable, make logged tasks re-trackable from Timer view.

---

## 7. Scope A Detailed Plan (Correctness)

## 7.1 Core principle

For Jira submission, aggregate only entries where `syncedToJira === false`.

Never infer submittability from a task-level "any synced" flag.

## 7.2 Store-layer changes

In `entries` store:

1. Keep existing aggregate for display compatibility.
2. Add explicit unlogged aggregate function, for example:
   - `getUnloggedAggregatedEntries()`
3. Ensure this function:
   - includes only `!syncedToJira`
   - preserves `entryIds` = unsynced ids only
   - computes `totalSecs` from unsynced entries only
   - computes running state only from unsynced running entry (if any)

## 7.3 LogFlow changes

Replace all selection/submission sources from generic aggregate to unlogged aggregate:

1. Initial auto-select list
2. Display list
3. Selected count and selected duration
4. Submission payload (`entryIds`, duration, started)

Expected result:

- If user logs `ABC-123`, tracks again, and reopens LogFlow, new unsynced segment appears correctly.

## 7.4 SuccessScreen changes

`Still Tracking` should derive from unlogged aggregate, not mixed aggregate, to avoid misleading rows.

## 7.5 Backend impact

No backend contract change required:

- `submit_batch_worklog` already marks by entry id.
- As long as frontend passes correct unsynced `entryIds`, backend remains valid.

## 7.6 Risk

Medium (submission logic), but bounded and testable.

---

## 8. Scope B Detailed Plan (Retracking UX)

## 8.1 Behavior options

### Option 1 (recommended, minimal change)

Keep `Logged Today` section, but make those rows trackable again:

- Show play button for logged rows.
- Clicking row or play starts timer as normal.
- Keep a subtle "already logged" badge or styling.

Pros:

- Minimal re-grouping risk.
- Users still see logged status.

### Option 2

Move logged tasks back into My Tasks/Unlogged with additional status labels.

Cons:

- More churn in grouping logic.
- Higher visual regression risk.

## 8.2 Recommended implementation details

1. Update `TaskCard`:
   - Remove `if (logged) return` guard for click handler or add a new prop to allow retrack.
   - Preserve logged styling but keep action enabled.
2. Update Timer grouping:
   - Do not exclude tasks from actionable groups solely because they have any logged entry.
   - Prefer grouping by "has active timer / has unsynced stopped entries / other tasks", and keep separate historical "Logged Today" summary if needed.

## 8.3 Risk

Low to medium (mostly UI/state flow).

---

## 9. Impact Analysis

## 9.1 User behavior changes

- Users can re-start a task after logging.
- Task can appear as both "already logged earlier" and "currently tracking/new unlogged work."

## 9.2 Data integrity

- Improved integrity if Scope A is applied:
  - prevents accidental omission of unsynced follow-up work
  - prevents duplicate inclusion of already synced entry ids

## 9.3 Performance

- Negligible. Aggregation is in-memory over today entries.

## 9.4 Backward compatibility

- No DB migration.
- No API change.
- Existing historical entries remain intact.

---

## 10. Edge Cases and Expected Outcomes

1. Logged task retracked immediately:
   - New entry created, appears as trackable.
   - LogFlow shows only new unsynced entry.

2. Task has mixed entries (some synced, some unsynced):
   - LogFlow includes task with unsynced duration only.

3. Running timer on previously logged task:
   - Timer view shows active state correctly.
   - Stop then LogFlow includes that new stopped segment.

4. Partial Jira submit failure:
   - Successful entry ids marked synced.
   - Failed ids remain unsynced and retryable.

5. Multiple submissions same day:
   - Each submission includes only unsynced ids at submission time.

6. App sleep/force-close while retracking:
   - Existing timer robustness still applies.
   - Downtime excluded by power/heartbeat mechanisms.

7. Round-to-15m enabled:
   - Submission duration follows selected rounding.
   - UI should clearly display rounded vs raw for selected set.

8. Manual edit of previously synced entry:
   - Current system likely keeps `syncedToJira=true`.
   - Out-of-scope for this change unless "reopen synced entry" behavior is desired.

---

## 11. Risk Register

1. Risk: Duplicate submission from mixed aggregate.
   - Severity: High
   - Mitigation: Scope A mandatory before Scope B.

2. Risk: UI confusion if same task appears in multiple groups.
   - Severity: Medium
   - Mitigation: explicit labels/badges, deterministic grouping rules.

3. Risk: Regression in success screen "still tracking" accuracy.
   - Severity: Medium
   - Mitigation: source `Still Tracking` from unlogged aggregate only.

4. Risk: Cross-tab totals mismatch (Today/Timer/LogFlow).
   - Severity: Medium
   - Mitigation: define and document metric intent per screen.

---

## 12. Validation Plan

## 12.1 Manual QA matrix (minimum)

1. Fresh day, track A, log A, track A again, stop, log again.
2. Track A + B, log only A, continue B, verify B unaffected.
3. Log A, then start A and B, stop both, log both, ensure unsynced only.
4. Simulate Jira failure for one task in batch, retry flow.
5. Sleep/wake during second tracking period, verify no jump.
6. Force-close/reopen during second tracking period, verify no jump.

## 12.2 Data-level checks

Verify in DB:

- `synced_to_jira` flips only for submitted entry ids.
- New sessions create new row ids and are independently syncable.
- No previously synced row gets resubmitted unless explicitly designed.

## 12.3 Automated tests (recommended to add)

1. Store unit test: mixed synced/unsynced entries aggregate correctly for unlogged selector.
2. LogFlow unit test: submission payload includes unsynced ids only.
3. Integration test: second log same task/day submits only new entry ids.

---

## 13. Rollout Approach

1. Implement Scope A in one PR (no UI behavior change for retrack yet).
2. Validate with QA matrix + regression checks.
3. Implement Scope B in second PR.
4. Optional: add temporary debug logs around submission payload and selected ids during rollout.

Rollback:

- If Scope B introduces UX regression, revert Scope B only.
- Keep Scope A because it improves correctness independently.

---

## 14. Implementation Task Breakdown (Later Execution)

## A. Correctness PR

1. Add unlogged aggregate selector in entries store.
2. Refactor LogFlow to use unlogged selector for list/selection/submit.
3. Refactor SuccessScreen `Still Tracking` to unlogged source.
4. Add tests for aggregation and payload construction.

## B. Retrack UX PR

1. Make logged task card action-enabled (or dedicated retrack button).
2. Update Timer grouping rules to keep tasks trackable after logging.
3. Add small UI cue: "Logged earlier today" badge to avoid confusion.
4. QA pass for grouping and click behavior.

---

## 15. Final Recommendation

Proceed with **Scope A first** (correctness), then **Scope B** (UX retrack enable).

This gives the safest path with minimal risk of duplicate/missed Jira logs and keeps implementation manageable.
