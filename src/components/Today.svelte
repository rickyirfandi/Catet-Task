<script lang="ts">
  import { getAggregatedEntries, getTotalSecs, refresh } from '$lib/stores/entries.svelte';
  import type { AggregatedEntry } from '$lib/stores/entries.svelte';
  import { getTasks } from '$lib/stores/tasks.svelte';
  import { getElapsedSecs, getTaskId, getStatus } from '$lib/stores/timer.svelte';
  import { formatDurationShort, formatDateHeader } from '$lib/utils/time';
  import { onMount } from 'svelte';

  let showLogFlow = $state(false);

  onMount(() => { refresh(); });

  let aggEntries = $derived(getAggregatedEntries());
  let baseTotalSecs = $derived(getTotalSecs());
  let tasks = $derived(getTasks());
  let dateHeader = $derived(formatDateHeader(new Date().toISOString()));
  let timerTaskId = $derived(getTaskId());
  let timerStatus = $derived(getStatus());
  let liveElapsed = $derived(timerStatus !== 'idle' ? getElapsedSecs() : 0);
  let totalSecs = $derived(baseTotalSecs + liveElapsed);

  function entryDuration(entry: AggregatedEntry): number {
    if (entry.isRunning && timerStatus !== 'idle' && timerTaskId === entry.taskId) {
      return entry.totalSecs + liveElapsed;
    }
    return entry.totalSecs;
  }

  const colors = ['var(--accent-blue)', 'var(--accent-purple)', 'var(--accent-orange)', 'var(--accent-green)', 'var(--accent-red)'];

  function taskName(taskId: string): string {
    return tasks.find(t => t.id === taskId)?.summary ?? taskId;
  }

  function openLogFlow() {
    // Will be handled by dispatching event to parent
    showLogFlow = true;
  }
</script>

{#if showLogFlow}
  {#await import('./LogFlow.svelte') then { default: LogFlow }}
    <LogFlow onclose={() => showLogFlow = false} />
  {/await}
{:else}
  <div class="today">
    <div class="today-header">
      <div class="date-label">{dateHeader}</div>
      <div class="total-row">
        <span class="total-time">{formatDurationShort(totalSecs)}</span>
        <span class="total-label">tracked today</span>
      </div>
    </div>

    {#if aggEntries.length > 0}
      <div class="dur-bar">
        {#each aggEntries as entry, i}
          {@const secs = entryDuration(entry)}
          {@const pct = totalSecs > 0 ? (secs / totalSecs) * 100 : 0}
          <div class="dur-seg" style="width:{pct}%;background:{colors[i % colors.length]}"></div>
        {/each}
      </div>
    {/if}

    <div class="entry-list">
      {#each aggEntries as entry, i (entry.taskId)}
        <div class="entry-row">
          <div class="entry-color" style="background:{colors[i % colors.length]}"></div>
          <div class="entry-info">
            <div class="entry-key">
              {entry.taskId}
              {#if entry.isRunning && timerStatus !== 'idle'}
                <span class="live-badge">&#9679; LIVE</span>
              {/if}
            </div>
            <div class="entry-name">{taskName(entry.taskId)}</div>
          </div>
          <span class="entry-dur" class:live={entry.isRunning && timerStatus !== 'idle'}>
            {formatDurationShort(entryDuration(entry))}
          </span>
        </div>
      {/each}
    </div>

    <div class="today-footer">
      <button class="btn-log" onclick={openLogFlow}>&#128640; Log to Jira</button>
      <button class="btn-export">Export</button>
    </div>
  </div>
{/if}

<style>
  .today {
    display: flex;
    flex-direction: column;
  }

  .today-header {
    padding: 16px 18px 10px;
  }

  .date-label {
    font-size: 12px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    margin-bottom: 8px;
  }

  .total-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .total-time {
    font-size: 32px;
    font-weight: 700;
    font-family: var(--font-mono);
  }

  .total-label {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .dur-bar {
    margin: 0 18px 14px;
    height: 8px;
    border-radius: 4px;
    background: var(--bg-card);
    display: flex;
    overflow: hidden;
    gap: 2px;
  }

  .dur-seg {
    height: 100%;
    border-radius: 3px;
  }

  .entry-list {
    padding: 0 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .entry-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .entry-color {
    width: 4px;
    height: 32px;
    border-radius: 2px;
    flex-shrink: 0;
  }

  .entry-info {
    flex: 1;
    min-width: 0;
  }

  .entry-key {
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--text-secondary);
  }

  .live-badge {
    color: var(--accent-green);
    font-size: 9px;
  }

  .entry-name {
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .entry-dur {
    font-size: 14px;
    font-weight: 600;
    font-family: var(--font-mono);
    flex-shrink: 0;
  }

  .entry-dur.live {
    color: var(--accent-green);
  }

  .today-footer {
    padding: 4px 14px 16px;
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

  .btn-log:hover {
    opacity: 0.9;
  }

  .btn-export {
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
