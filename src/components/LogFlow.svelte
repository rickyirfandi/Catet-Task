<script lang="ts">
  import Checkbox from './shared/Checkbox.svelte';
  import Badge from './shared/Badge.svelte';
  import SubmitProgress from './SubmitProgress.svelte';
  import SuccessScreen from './SuccessScreen.svelte';
  import { getEntries, getAggregatedEntries, refresh } from '$lib/stores/entries.svelte';
  import type { AggregatedEntry } from '$lib/stores/entries.svelte';
  import { getTasks } from '$lib/stores/tasks.svelte';
  import { getTaskId as getActiveTimerTaskId } from '$lib/stores/timer.svelte';
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
  let selectedTaskIds = $state<Set<string>>(new Set());

  onMount(() => {
    refresh().then(() => {
      // Default: select all stopped (non-running) tasks
      const agg = getAggregatedEntries().filter(e => !e.isSynced);
      const stopped = agg.filter(e => !e.isRunning);
      selectedTaskIds = new Set(stopped.map(e => e.taskId));
    });
  });

  // Aggregated entries (one per task), exclude already synced
  let aggEntries = $derived(getAggregatedEntries().filter(e => !e.isSynced));
  let rawEntries = $derived(getEntries());
  let tasks = $derived(getTasks());
  let activeTimerTaskId = $derived(getActiveTimerTaskId());
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
    const submissions = selected.map(e => {
      // Find the earliest entry's start time for this task
      const taskEntries = rawEntries.filter(r => r.taskId === e.taskId && !r.syncedToJira);
      const earliest = taskEntries.reduce((a, b) => a.startTime < b.startTime ? a : b, taskEntries[0]);
      return {
        entryIds: e.entryIds,
        taskId: e.taskId,
        timeSpentSeconds: getRoundedSecs(e),
        started: earliest?.startTime ?? e.latestStartTime,
        comment: '',
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
    tasks={aggEntries.filter(e => selectedTaskIds.has(e.taskId)).map(e => ({ taskId: e.taskId, totalSecs: getRoundedSecs(e) }))}
    results={submitResults}
  />
{:else if step === 'result'}
  <SuccessScreen
    tasks={aggEntries.filter(e => selectedTaskIds.has(e.taskId)).map(e => ({ taskId: e.taskId, totalSecs: getRoundedSecs(e) }))}
    results={submitResults}
    onclose={onclose}
  />
{:else}
  <div class="logflow">
    <header class="p-header">
      <div class="p-header-left">
        <button class="back-btn" onclick={onclose}>&larr;</button>
        <div>
          <div class="p-title">Log to Jira</div>
          <div class="p-sub">{dateHeader}</div>
        </div>
      </div>
    </header>

    <div class="summary-bar">
      <div class="sum-total">
        <span class="sum-time">{formatDurationShort(selectedSecs)}</span>
        <span class="sum-label">selected <span class="muted">/ {formatDurationShort(totalSecs)}</span></span>
      </div>
      <span class="sum-count">{selectedCount} of {aggEntries.length}</span>
    </div>

    <div class="select-bar">
      <div class="select-left">
        <Checkbox checked={allSelected} partial={someSelected} onchange={() => allSelected ? doSelectNone() : doSelectAll()} />
        <span class="select-label">{selectedCount} selected</span>
      </div>
      <div class="select-links">
        <button class="link" onclick={doSelectAll}>All</button>
        <span class="sep">&middot;</span>
        <button class="link" onclick={doSelectNone}>None</button>
        <span class="sep">&middot;</span>
        <button class="link" onclick={doSelectStopped}>Stopped</button>
      </div>
    </div>

    <div class="round-bar">
      <span class="round-label">&#9201; Round to</span>
      <div class="round-chips">
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
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="r-item" class:selected class:deselected={!selected}>
          <div class="item-cb-wrap" onclick={() => toggleTask(entry.taskId)}>
            <Checkbox checked={selected} />
          </div>
          <div class="item-body">
            <div class="r-top">
              <span class="task-key">{entry.taskId}</span>
              {#if entry.isRunning}
                <Badge variant="running" />
              {:else if isRounded(entry)}
                <Badge variant="rounded" />
              {/if}
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
          </div>
        </div>
      {/each}
    </div>

    {#if aggEntries.some(e => e.isRunning)}
      <div class="callout yellow">
        <span class="callout-icon">&#128161;</span>
        <div class="callout-text">
          <strong>{aggEntries.find(e => e.isRunning)?.taskId}</strong> is still running. Skipped tasks keep their tracked time for your next log session.
        </div>
      </div>
    {/if}

    <div class="panel-footer">
      <div class="footer-info">
        <span class="footer-sel"><strong>{selectedCount}</strong> tasks &middot; <strong>{formatDurationShort(selectedSecs)}</strong></span>
        <span class="footer-rem">{aggEntries.length - selectedCount} kept for later</span>
      </div>
      <div class="footer-btns">
        <button class="btn-log" class:disabled={selectedCount === 0} disabled={selectedCount === 0} onclick={handleSubmit}>
          &#128640; Log Selected ({selectedCount})
        </button>
        <button class="btn-cancel" onclick={onclose}>Cancel</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .logflow {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .p-header {
    padding: 16px 18px 12px;
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
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: 14px;
    cursor: pointer;
  }

  .p-title {
    font-size: 15px;
    font-weight: 600;
  }

  .p-sub {
    font-size: 11px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
  }

  .summary-bar {
    padding: 14px 18px;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .sum-total {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .sum-time {
    font-size: 22px;
    font-weight: 700;
    font-family: var(--font-mono);
  }

  .sum-label {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .sum-label .muted {
    color: var(--text-muted);
  }

  .sum-count {
    font-size: 11px;
    color: var(--text-secondary);
    background: var(--bg-panel);
    padding: 4px 10px;
    border-radius: 20px;
    font-family: var(--font-mono);
  }

  .select-bar {
    padding: 10px 18px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border);
  }

  .select-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .select-label {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .select-links {
    display: flex;
    gap: 8px;
    font-size: 11px;
    font-family: var(--font-mono);
    font-weight: 500;
  }

  .link {
    color: var(--accent-blue);
    cursor: pointer;
    background: none;
    border: none;
    font-size: 11px;
    font-family: var(--font-mono);
    font-weight: 500;
  }

  .sep {
    color: var(--text-muted);
  }

  .round-bar {
    padding: 10px 18px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border);
  }

  .round-label {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .round-chips {
    display: flex;
    gap: 4px;
  }

  .rc {
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-mono);
    padding: 4px 10px;
    border-radius: 4px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
  }

  .rc.active {
    background: rgba(61, 122, 237, 0.12);
    border-color: var(--accent-blue);
    color: var(--accent-blue);
  }

  .review-list {
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    flex: 1;
    overflow-y: auto;
  }

  .r-item {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 11px 13px;
    display: flex;
    gap: 11px;
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
    opacity: 0.45;
  }

  .r-item.deselected:hover {
    opacity: 0.7;
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
    margin-bottom: 5px;
  }

  .task-key {
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--accent-blue);
  }

  .r-name {
    font-size: 13px;
    font-weight: 500;
    line-height: 1.35;
    margin-bottom: 7px;
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
    font-size: 14px;
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

  .callout {
    margin: 0 12px 10px;
    padding: 11px 13px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }

  .callout.yellow {
    background: var(--accent-yellow-dim);
    border: 1px solid rgba(250, 204, 21, 0.2);
  }

  .callout-icon {
    font-size: 15px;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .callout-text {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .callout-text :global(strong) {
    color: var(--accent-orange);
    font-weight: 600;
  }

  .panel-footer {
    border-top: 1px solid var(--border);
    padding: 14px 18px;
    background: var(--bg-panel);
  }

  .footer-info {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  .footer-sel {
    font-size: 12px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
  }

  .footer-sel :global(strong) {
    color: var(--text-primary);
    font-weight: 600;
  }

  .footer-rem {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .footer-btns {
    display: flex;
    gap: 8px;
  }

  .btn-log {
    flex: 1;
    background: linear-gradient(135deg, var(--accent-green), #22b88a);
    border: none;
    border-radius: var(--radius-sm);
    padding: 12px;
    font-size: 13px;
    font-weight: 600;
    color: #0d0f13;
    cursor: pointer;
    font-family: var(--font-body);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
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
    padding: 12px 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    font-family: var(--font-body);
  }
</style>
