<script lang="ts">
  import { onMount } from 'svelte';
  import type { Task, TaskDetailData, TimeEntry } from '$lib/types';
  import { getStatus, getTaskId, getElapsedSecs, toggle } from '$lib/stores/timer.svelte';
  import { getEntries } from '$lib/stores/entries.svelte';
  import { togglePin } from '$lib/stores/tasks.svelte';
  import { getTaskDetail } from '$lib/api/tauri';
  import Badge from './shared/Badge.svelte';
  import { formatDateTime, formatDuration, formatTime, getLocalTimezoneLabel } from '$lib/utils/time';

  let { task, onBack }: { task: Task; onBack: () => void } = $props();

  let isActive = $derived(getTaskId() === task.id && getStatus() !== 'idle');
  let isRunning = $derived(getTaskId() === task.id && getStatus() === 'running');
  let isPaused = $derived(getTaskId() === task.id && getStatus() === 'paused');

  let taskEntries = $derived(getEntries().filter((e: TimeEntry) => e.taskId === task.id));
  let timezone = $derived(getLocalTimezoneLabel());

  let detail = $state<TaskDetailData | null>(null);
  let detailLoading = $state(false);
  let detailError = $state('');

  let totalSecs = $derived.by(() => {
    let sum = 0;
    for (const e of taskEntries) {
      if (e.endTime === null) {
        if (isActive) sum += getElapsedSecs();
      } else {
        sum += e.adjustedSecs ?? e.durationSecs ?? 0;
      }
    }
    return sum;
  });

  let statusBadge = $derived.by((): 'progress' | 'review' | 'logged' | 'todo' => {
    const s = task.status?.toLowerCase() ?? '';
    if (s.includes('progress')) return 'progress';
    if (s.includes('review')) return 'review';
    if (s.includes('done') || s.includes('closed')) return 'logged';
    return 'todo';
  });

  function segmentDuration(entry: TimeEntry): number {
    if (entry.endTime === null) return isActive ? getElapsedSecs() : 0;
    return entry.adjustedSecs ?? entry.durationSecs ?? 0;
  }

  async function loadTaskDetail() {
    detailLoading = true;
    detailError = '';
    try {
      detail = await getTaskDetail(task.id);
    } catch (e: any) {
      detail = null;
      detailError = typeof e === 'string' ? e : e?.message || 'Failed to load task detail';
    } finally {
      detailLoading = false;
    }
  }

  onMount(() => {
    loadTaskDetail();
  });
</script>

<div class="detail">
  <button class="back-btn" onclick={onBack}>&larr; Back</button>

  <div class="task-header">
    <div class="task-key-row">
      <span class="task-key">{task.id}</span>
      <Badge variant={statusBadge} label={task.status} />
    </div>
    <div class="task-summary">{task.summary}</div>
    <div class="task-meta">
      {task.projectName || task.projectKey}{task.sprintName ? ` - ${task.sprintName}` : ''}
    </div>
  </div>

  <div class="time-section">
    <span class="time-label">TODAY'S TIME</span>
    <span class="time-total">{formatDuration(totalSecs)}</span>
  </div>

  <div class="detail-meta">
    <div class="detail-meta-label">TASK DETAIL</div>
    {#if detailLoading}
      <div class="detail-row muted">Loading task metadata...</div>
    {:else if detailError}
      <div class="detail-row muted">{detailError}</div>
    {:else if detail}
      <div class="detail-grid">
        <div class="detail-row"><span class="k">Type</span><span class="v">{detail.issueType ?? '-'}</span></div>
        <div class="detail-row"><span class="k">Priority</span><span class="v">{detail.priority ?? '-'}</span></div>
        <div class="detail-row"><span class="k">Assignee</span><span class="v">{detail.assignee ?? '-'}</span></div>
        <div class="detail-row"><span class="k">Created</span><span class="v">{detail.createdAt ? formatDateTime(detail.createdAt) : '-'}</span></div>
        <div class="detail-row"><span class="k">Updated</span><span class="v">{detail.updatedAt ? formatDateTime(detail.updatedAt) : '-'}</span></div>
      </div>
      {#if detail.description}
        <div class="desc">
          <div class="k">Description</div>
          <div class="desc-text">{detail.description}</div>
        </div>
      {/if}
    {/if}
  </div>

  <!-- svelte-ignore a11y_consider_explicit_label -->
  <button
    class="play-btn"
    class:running={isRunning}
    class:paused={isPaused}
    onclick={() => toggle(task.id)}
  >
    {#if isRunning}Pause{:else if isPaused}Resume{:else}Start Timer{/if}
  </button>

  {#if taskEntries.length > 0}
    <div class="segments">
      <div class="segments-label">TIME SEGMENTS - LOCAL ({timezone})</div>
      {#each taskEntries as entry (entry.id)}
        <div class="segment" class:synced={entry.syncedToJira}>
          <div class="seg-range">
            <span>{formatTime(entry.startTime)}</span>
            <span class="seg-sep">-</span>
            {#if entry.endTime}
              <span>{formatTime(entry.endTime)}</span>
            {:else}
              <span class="seg-now">now</span>
            {/if}
          </div>
          <div class="seg-right">
            <span class="seg-dur">{formatDuration(segmentDuration(entry))}</span>
            {#if entry.syncedToJira}
              <span class="seg-check">sync</span>
            {:else if entry.endTime === null}
              <span class="seg-live">live</span>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty">No time tracked today</div>
  {/if}

  <!-- svelte-ignore a11y_consider_explicit_label -->
  <button class="pin-btn" onclick={() => togglePin(task.id)}>
    {task.pinned ? 'Unpin' : 'Pin'}
  </button>
</div>

<style>
  .detail {
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 0 12px 14px;
    height: 100%;
  }

  .back-btn {
    align-self: flex-start;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 12px;
    font-family: var(--font-mono);
    cursor: pointer;
    padding: 8px 4px;
    margin-bottom: 2px;
    transition: color 0.15s;
  }

  .back-btn:hover {
    color: var(--text-primary);
  }

  .task-header {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px 14px;
    margin-bottom: 8px;
  }

  .task-key-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .task-key {
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--accent-blue);
    letter-spacing: 0.5px;
  }

  .task-summary {
    font-size: 13px;
    font-weight: 500;
    line-height: 1.4;
    color: var(--text-primary);
    margin-bottom: 6px;
  }

  .task-meta {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .time-section {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    margin-bottom: 8px;
  }

  .time-label {
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-mono);
    letter-spacing: 1.5px;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .time-total {
    font-size: 20px;
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--accent-green);
    letter-spacing: 1px;
  }

  .detail-meta {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 12px;
    margin-bottom: 8px;
  }

  .detail-meta-label {
    font-size: 9px;
    font-weight: 600;
    font-family: var(--font-mono);
    letter-spacing: 1.5px;
    color: var(--text-muted);
    text-transform: uppercase;
    margin-bottom: 6px;
  }

  .detail-grid {
    display: grid;
    gap: 4px;
  }

  .detail-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    font-size: 11px;
  }

  .detail-row .k,
  .desc .k {
    color: var(--text-muted);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.8px;
    font-size: 10px;
  }

  .detail-row .v {
    color: var(--text-primary);
    font-family: var(--font-mono);
    text-align: right;
  }

  .detail-row.muted {
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .desc {
    margin-top: 8px;
    border-top: 1px solid var(--border);
    padding-top: 8px;
  }

  .desc-text {
    margin-top: 4px;
    white-space: pre-wrap;
    line-height: 1.4;
    color: var(--text-primary);
    font-size: 12px;
  }

  .play-btn {
    width: 100%;
    padding: 11px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 600;
    font-family: var(--font-body);
    cursor: pointer;
    transition: all 0.15s;
    margin-bottom: 8px;
  }

  .play-btn:hover {
    background: var(--bg-card-hover);
    border-color: var(--accent-blue);
    color: var(--accent-blue);
  }

  .play-btn.running {
    background: rgba(45, 212, 160, 0.08);
    border-color: var(--accent-green);
    color: var(--accent-green);
  }

  .play-btn.paused {
    background: rgba(61, 122, 237, 0.08);
    border-color: var(--accent-blue);
    color: var(--accent-blue);
  }

  .segments {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    margin-bottom: 8px;
  }

  .segments-label {
    font-size: 9px;
    font-weight: 600;
    font-family: var(--font-mono);
    letter-spacing: 1.5px;
    color: var(--text-muted);
    text-transform: uppercase;
    padding: 8px 12px 6px;
    border-bottom: 1px solid var(--border);
  }

  .segment {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    transition: background 0.1s;
  }

  .segment:last-child {
    border-bottom: none;
  }

  .segment.synced {
    opacity: 0.5;
  }

  .seg-range {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
  }

  .seg-sep {
    color: var(--text-muted);
  }

  .seg-now {
    color: var(--accent-green);
    font-style: italic;
  }

  .seg-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .seg-dur {
    font-size: 12px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--text-primary);
  }

  .seg-check,
  .seg-live {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    color: var(--accent-green);
  }

  .seg-live {
    animation: pulse-live 1.5s ease-in-out infinite;
  }

  @keyframes pulse-live {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }

  .empty {
    font-size: 12px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    text-align: center;
    padding: 20px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    margin-bottom: 8px;
  }

  .pin-btn {
    align-self: flex-start;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 12px;
    font-family: var(--font-mono);
    cursor: pointer;
    padding: 6px 4px;
    transition: color 0.15s;
  }

  .pin-btn:hover {
    color: var(--accent-orange);
  }
</style>
