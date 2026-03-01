<script lang="ts">
  import type { Task, TimeEntry } from '$lib/types';
  import { getStatus, getTaskId, getElapsedSecs, toggle } from '$lib/stores/timer.svelte';
  import { getEntries } from '$lib/stores/entries.svelte';
  import { togglePin } from '$lib/stores/tasks.svelte';
  import Badge from './shared/Badge.svelte';
  import { formatDuration, formatTime } from '$lib/utils/time';

  let { task, onBack }: { task: Task; onBack: () => void } = $props();

  let isActive   = $derived(getTaskId() === task.id && getStatus() !== 'idle');
  let isRunning  = $derived(getTaskId() === task.id && getStatus() === 'running');
  let isPaused   = $derived(getTaskId() === task.id && getStatus() === 'paused');

  let taskEntries = $derived(getEntries().filter((e: TimeEntry) => e.taskId === task.id));

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
</script>

<div class="detail">
  <button class="back-btn" onclick={onBack}>← Back</button>

  <div class="task-header">
    <div class="task-key-row">
      <span class="task-key">{task.id}</span>
      <Badge variant={statusBadge} label={task.status} />
    </div>
    <div class="task-summary">{task.summary}</div>
    <div class="task-meta">
      {task.projectName || task.projectKey}{task.sprintName ? ` · ${task.sprintName}` : ''}
    </div>
  </div>

  <div class="time-section">
    <span class="time-label">TODAY'S TIME</span>
    <span class="time-total">{formatDuration(totalSecs)}</span>
  </div>

  <!-- svelte-ignore a11y_consider_explicit_label -->
  <button
    class="play-btn"
    class:running={isRunning}
    class:paused={isPaused}
    onclick={() => toggle(task.id)}
  >
    {#if isRunning}⏸ Pause{:else if isPaused}▶ Resume{:else}▶ Start Timer{/if}
  </button>

  {#if taskEntries.length > 0}
    <div class="segments">
      <div class="segments-label">TIME SEGMENTS</div>
      {#each taskEntries as entry (entry.id)}
        <div class="segment" class:synced={entry.syncedToJira}>
          <div class="seg-range">
            <span>{formatTime(entry.startTime)}</span>
            <span class="seg-sep">–</span>
            {#if entry.endTime}
              <span>{formatTime(entry.endTime)}</span>
            {:else}
              <span class="seg-now">now</span>
            {/if}
          </div>
          <div class="seg-right">
            <span class="seg-dur">{formatDuration(segmentDuration(entry))}</span>
            {#if entry.syncedToJira}
              <span class="seg-check">✓</span>
            {:else if entry.endTime === null}
              <span class="seg-live">●</span>
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
    {task.pinned ? '★ Unpin' : '☆ Pin'}
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

  .seg-check {
    font-size: 11px;
    color: var(--accent-green);
  }

  .seg-live {
    font-size: 8px;
    color: var(--accent-green);
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
