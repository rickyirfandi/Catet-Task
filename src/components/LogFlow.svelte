<script lang="ts">
  import Checkbox from './shared/Checkbox.svelte';
  import Badge from './shared/Badge.svelte';
  import SubmitProgress from './SubmitProgress.svelte';
  import SuccessScreen from './SuccessScreen.svelte';
  import { getEntries, getAggregatedEntries, refresh } from '$lib/stores/entries.svelte';
  import type { AggregatedEntry } from '$lib/stores/entries.svelte';
  import { getTasks } from '$lib/stores/tasks.svelte';
  import { formatDurationShort, formatDateHeader } from '$lib/utils/time';
  import { roundToNearest } from '$lib/utils/rounding';
  import { submitBatchWorklog } from '$lib/api/tauri';
  import { listen } from '@tauri-apps/api/event';
  import type { LogFlowStep, WorklogProgress } from '$lib/types';
  import { onMount } from 'svelte';

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  let step = $state<LogFlowStep>('select');
  let roundIncrement = $state(15);
  let submitResults = $state<WorklogProgress[]>([]);
  let submittedTasks = $state<{ taskId: string; totalSecs: number }[]>([]);
  let selectedTaskIds = $state<Set<string>>(new Set());
  let comments = $state<Record<string, string>>({});
  let expandedTaskId = $state<string | null>(null);

  function toggleExpand(taskId: string) {
    expandedTaskId = expandedTaskId === taskId ? null : taskId;
  }

  function setComment(taskId: string, value: string) {
    comments = { ...comments, [taskId]: value };
  }

  onMount(() => {
    refresh().then(() => {
      // Select all unsynced tasks (timer is guaranteed stopped before entering LogFlow)
      const agg = getAggregatedEntries().filter(e => !e.isSynced);
      selectedTaskIds = new Set(agg.map(e => e.taskId));
    });
  });

  // Aggregated entries (one per task), exclude already synced
  let aggEntries = $derived(getAggregatedEntries().filter(e => !e.isSynced));
  let rawEntries = $derived(getEntries());
  let tasks = $derived(getTasks());
  let dateHeader = $derived(formatDateHeader(new Date().toISOString()));

  let totalSecs = $derived(aggEntries.reduce((s, e) => s + e.totalSecs, 0));
  let selectedSecs = $derived(aggEntries.filter(e => selectedTaskIds.has(e.taskId)).reduce((s, e) => s + getRoundedSecs(e), 0));
  let selectedCount = $derived(aggEntries.filter(e => selectedTaskIds.has(e.taskId)).length);
  let allSelected = $derived(selectedCount === aggEntries.length && aggEntries.length > 0);
  let someSelected = $derived(selectedCount > 0 && selectedCount < aggEntries.length);

  function taskName(taskId: string) {
    return tasks.find(t => t.id === taskId)?.summary ?? taskId;
  }

  function getRoundedSecs(entry: AggregatedEntry): number {
    return roundIncrement > 1 ? roundToNearest(entry.totalSecs, roundIncrement) : entry.totalSecs;
  }

  function isRounded(entry: AggregatedEntry): boolean {
    return roundIncrement > 1 && getRoundedSecs(entry) !== entry.totalSecs;
  }

  function toggleTask(taskId: string) {
    const next = new Set(selectedTaskIds);
    if (next.has(taskId)) next.delete(taskId); else next.add(taskId);
    selectedTaskIds = next;
  }

  function doSelectAll() {
    selectedTaskIds = new Set(aggEntries.map(e => e.taskId));
  }

  function doSelectNone() {
    selectedTaskIds = new Set();
  }

  function doSelectStopped() {
    selectedTaskIds = new Set(aggEntries.filter(e => !e.isRunning).map(e => e.taskId));
  }

  async function handleSubmit() {
    step = 'submitting';
    submitResults = [];

    const selected = aggEntries.filter(e => selectedTaskIds.has(e.taskId));
    submittedTasks = selected.map(e => ({ taskId: e.taskId, totalSecs: getRoundedSecs(e) }));
    const submissions = selected.map(e => {
      // Find the earliest entry's start time for this task
      const taskEntries = rawEntries.filter(r => r.taskId === e.taskId && !r.syncedToJira);
      const earliest = taskEntries.reduce((a, b) => a.startTime < b.startTime ? a : b, taskEntries[0]);
      return {
        entryIds: e.entryIds,
        taskId: e.taskId,
        timeSpentSeconds: getRoundedSecs(e),
        started: earliest?.startTime ?? e.latestStartTime,
        comment: comments[e.taskId]?.trim() ?? '',
      };
    });

    const unlisten = await listen<WorklogProgress>('worklog-progress', (event) => {
      submitResults = [...submitResults, event.payload];
    });

    try {
      await submitBatchWorklog(submissions);
    } catch (err) {
      console.error('Batch submit failed:', err);
    }

    unlisten();
    step = 'result';
    refresh();
  }
</script>

{#if step === 'submitting'}
  <SubmitProgress
    tasks={submittedTasks}
    results={submitResults}
  />
{:else if step === 'result'}
  <SuccessScreen
    tasks={submittedTasks}
    results={submitResults}
    onclose={onclose}
  />
{:else}
  <div class="logflow">
    <header class="p-header">
      <div class="p-header-left">
        <button class="back-btn" onclick={onclose}>&larr;</button>
        <span class="p-title">Log to Jira</span>
      </div>
      <span class="p-date">{dateHeader}</span>
    </header>

    <div class="toolbar">
      <div class="tb-left">
        <Checkbox checked={allSelected} partial={someSelected} onchange={() => allSelected ? doSelectNone() : doSelectAll()} />
        <div class="tb-links">
          <button class="link" onclick={doSelectAll}>All</button>
          <span class="sep">&middot;</span>
          <button class="link" onclick={doSelectNone}>None</button>
          <span class="sep">&middot;</span>
          <button class="link" onclick={doSelectStopped}>Stopped</button>
        </div>
      </div>
      <div class="tb-chips">
        {#each [1, 15, 30] as inc}
          <button class="rc" class:active={roundIncrement === inc} onclick={() => roundIncrement = inc}>
            {inc}m
          </button>
        {/each}
      </div>
    </div>

    <div class="review-list">
      {#each aggEntries as entry (entry.taskId)}
        {@const selected = selectedTaskIds.has(entry.taskId)}
        {@const expanded = expandedTaskId === entry.taskId}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="r-item" class:selected class:deselected={!selected} class:expanded>
          <div class="item-cb-wrap" onclick={() => toggleTask(entry.taskId)}>
            <Checkbox checked={selected} />
          </div>
          <div class="item-body" onclick={() => toggleExpand(entry.taskId)}>
            <div class="r-top">
              <span class="task-key">{entry.taskId}</span>
              <div class="r-top-right">
                {#if isRounded(entry)}
                  <Badge variant="rounded" />
                {/if}
                <button class="edit-btn" class:has-comment={!!comments[entry.taskId]?.trim()}>&#9998;</button>
              </div>
            </div>
            <div class="r-name">{taskName(entry.taskId)}</div>
            <div class="r-meta">
              <div class="r-dur-group">
                <span class="r-dur">{formatDurationShort(getRoundedSecs(entry))}</span>
                {#if isRounded(entry)}
                  <span class="r-dur-orig">{formatDurationShort(entry.totalSecs)}</span>
                {/if}
              </div>
              <span class="r-sessions">{entry.entryIds.length} session{entry.entryIds.length > 1 ? 's' : ''}</span>
            </div>
            {#if comments[entry.taskId]?.trim() && !expanded}
              <div class="comment-preview">{comments[entry.taskId]}</div>
            {/if}
          </div>
          {#if expanded}
            <!-- svelte-ignore a11y_autofocus -->
            <div class="comment-wrap" onclick={(e) => e.stopPropagation()}>
              <textarea
                class="comment-input"
                placeholder="Add a worklog comment..."
                rows="2"
                autofocus
                value={comments[entry.taskId] ?? ''}
                oninput={(e) => setComment(entry.taskId, e.currentTarget.value)}
              ></textarea>
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <div class="panel-footer">
      <button class="btn-log" class:disabled={selectedCount === 0} disabled={selectedCount === 0} onclick={handleSubmit}>
        Log ({selectedCount}) &middot; {formatDurationShort(selectedSecs)}
      </button>
      <button class="btn-cancel" onclick={onclose}>Cancel</button>
    </div>
  </div>
{/if}

<style>
  .logflow {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  /* ── Header: single line ── */
  .p-header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .p-header-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .back-btn {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
  }

  .p-title {
    font-size: 14px;
    font-weight: 600;
  }

  .p-date {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  /* ── Toolbar: checkbox + links + round chips ── */
  .toolbar {
    padding: 8px 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border);
    background: var(--bg-card);
  }

  .tb-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .tb-links {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .link {
    color: var(--accent-blue);
    cursor: pointer;
    background: none;
    border: none;
    font-size: 10px;
    font-family: var(--font-mono);
    font-weight: 500;
    padding: 0;
  }

  .sep {
    color: var(--text-muted);
    font-size: 10px;
  }

  .tb-chips {
    display: flex;
    gap: 3px;
  }

  .rc {
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-mono);
    padding: 3px 8px;
    border-radius: 4px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
  }

  .rc.active {
    background: rgba(61, 122, 237, 0.12);
    border-color: var(--accent-blue);
    color: var(--accent-blue);
  }

  /* ── Task list ── */
  .review-list {
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    overflow-y: auto;
  }

  .r-item {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    cursor: pointer;
    transition: all 0.15s;
    text-align: left;
    width: 100%;
  }

  .r-item:hover {
    background: var(--bg-card-hover);
  }

  .r-item.selected {
    border-color: rgba(61, 122, 237, 0.4);
    background: rgba(61, 122, 237, 0.04);
  }

  .r-item.deselected {
    opacity: 0.4;
  }

  .r-item.deselected:hover {
    opacity: 0.65;
  }

  .item-cb-wrap {
    flex-shrink: 0;
    margin-top: 1px;
  }

  .item-body {
    flex: 1;
    min-width: 0;
  }

  .r-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 3px;
  }

  .r-top-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .task-key {
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--accent-blue);
  }

  .r-name {
    font-size: 12px;
    font-weight: 500;
    line-height: 1.3;
    margin-bottom: 5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .r-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .r-dur-group {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .r-dur {
    font-size: 13px;
    font-weight: 600;
    font-family: var(--font-mono);
  }

  .r-dur-orig {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    text-decoration: line-through;
  }

  .r-sessions {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  /* ── Edit button & comment ── */
  .edit-btn {
    font-size: 12px;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0 2px;
    line-height: 1;
  }

  .edit-btn.has-comment {
    color: var(--accent-blue);
  }

  .r-item.expanded {
    border-color: rgba(61, 122, 237, 0.4);
  }

  .comment-preview {
    font-size: 11px;
    color: var(--text-muted);
    font-style: italic;
    margin-top: 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .comment-wrap {
    width: 100%;
    padding-left: 0;
  }

  .comment-input {
    width: 100%;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-body);
    padding: 8px 10px;
    resize: vertical;
    min-height: 36px;
    outline: none;
  }

  .comment-input:focus {
    border-color: var(--accent-blue);
  }

  .comment-input::placeholder {
    color: var(--text-muted);
  }

  /* ── Footer: single row ── */
  .panel-footer {
    border-top: 1px solid var(--border);
    padding: 10px 16px;
    background: var(--bg-panel);
    display: flex;
    gap: 8px;
  }

  .btn-log {
    flex: 1;
    background: linear-gradient(135deg, var(--accent-green), #22b88a);
    border: none;
    border-radius: var(--radius-sm);
    padding: 11px;
    font-size: 13px;
    font-weight: 600;
    color: #0d0f13;
    cursor: pointer;
    font-family: var(--font-mono);
    text-align: center;
    box-shadow: 0 4px 16px rgba(45, 212, 160, 0.2);
  }

  .btn-log.disabled {
    opacity: 0.4;
    cursor: not-allowed;
    box-shadow: none;
  }

  .btn-cancel {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 11px 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    font-family: var(--font-body);
  }
</style>
