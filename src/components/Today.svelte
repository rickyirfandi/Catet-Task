<script lang="ts">
  import { getAggregatedEntries, getTotalSecs, getEntries, refresh } from '$lib/stores/entries.svelte';
  import type { AggregatedEntry } from '$lib/stores/entries.svelte';
  import { getTasks } from '$lib/stores/tasks.svelte';
  import { getElapsedSecs, getTaskId, getStatus, stop as stopTimer } from '$lib/stores/timer.svelte';
  import {
    formatDuration,
    formatDurationShort,
    formatDateHeader,
    getTodayLocalDateKey,
  } from '$lib/utils/time';
  import type { TimeEntry } from '$lib/types';
  import { onMount } from 'svelte';

  let showLogFlow = $state(false);
  let showStopConfirm = $state(false);
  let stopping = $state(false);
  let showExportPopup = $state(false);
  let exportDone = $state<string | null>(null);

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

  async function openLogFlow() {
    if (timerStatus === 'running') {
      showStopConfirm = true;
      return;
    }
    if (timerStatus === 'paused') {
      await stopTimer();
      await refresh();
    }
    showLogFlow = true;
  }

  async function stopAndLog() {
    stopping = true;
    try {
      await stopTimer();
      await refresh();
      showStopConfirm = false;
      showLogFlow = true;
    } catch (e) {
      console.error('Failed to stop timer:', e);
    }
    stopping = false;
  }

  function csvEscape(value: string): string {
    if (value.includes(',') || value.includes('"') || value.includes('\n')) {
      return '"' + value.replace(/"/g, '""') + '"';
    }
    return value;
  }

  function buildExportRows(): { headers: string[]; rows: string[][] } {
    const headers = ['Task ID', 'Summary', 'Duration'];
    const agg = getAggregatedEntries();
    const allTasks = getTasks();

    const rows = agg.map(entry => {
      const summary = allTasks.find(t => t.id === entry.taskId)?.summary ?? '';
      return [entry.taskId, summary, formatDuration(entry.totalSecs)];
    });
    return { headers, rows };
  }

  function buildReportText(): string {
    const generatedAt = new Intl.DateTimeFormat([], {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    }).format(new Date());

    const lines = [
      'Catet Task Report',
      `Period: ${dateHeader}`,
      `Generated: ${generatedAt}`,
      `Total tracked: ${formatDurationShort(totalSecs)}`,
      '',
      'Task breakdown:',
    ];

    if (aggEntries.length === 0) {
      lines.push('- No tracked entries today.');
      return lines.join('\n');
    }

    aggEntries.forEach((entry, i) => {
      const secs = entryDuration(entry);
      const pct = totalSecs > 0 ? ` (${Math.round((secs / totalSecs) * 100)}%)` : '';
      lines.push(
        `${i + 1}. ${entry.taskId} - ${taskName(entry.taskId)} - ${formatDurationShort(secs)}${pct}`
      );
    });

    return lines.join('\n');
  }

  function flashExportDone(msg: string) {
    exportDone = msg;
    setTimeout(() => {
      exportDone = null;
      showExportPopup = false;
    }, 1500);
  }

  async function copyToClipboard() {
    await navigator.clipboard.writeText(buildReportText());
    flashExportDone('Copied!');
  }

  function downloadCsv() {
    const { headers, rows } = buildExportRows();
    const lines = [
      headers.map(csvEscape).join(','),
      ...rows.map(r => r.map(csvEscape).join(',')),
    ];
    const blob = new Blob([lines.join('\n')], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const today = getTodayLocalDateKey();
    const a = document.createElement('a');
    a.href = url;
    a.download = `ct-export-${today}.csv`;
    a.click();
    URL.revokeObjectURL(url);
    flashExportDone('Downloaded!');
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
      {:else}
        <div class="empty-today">
          <div class="empty-today-icon">&#9201;</div>
          <div class="empty-today-text">No time tracked today</div>
          <div class="empty-today-hint">Start a timer from the Timer tab to see entries here.</div>
        </div>
      {/each}
    </div>

    <div class="today-footer">
      <button class="btn-log" onclick={openLogFlow}>&#128640; Log to Jira</button>
      <button class="btn-export" onclick={() => showExportPopup = true}>Export</button>
    </div>

    {#if showStopConfirm}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="dialog-overlay" onclick={() => showStopConfirm = false}>
        <div class="dialog" onclick={(e) => e.stopPropagation()}>
          <div class="dialog-msg">
            <strong>{timerTaskId}</strong> is still running.<br />Stop the timer to log worklogs.
          </div>
          <div class="dialog-btns">
            <button class="btn-stop-log" onclick={stopAndLog} disabled={stopping}>
              {stopping ? 'Stopping...' : 'Stop & Log'}
            </button>
            <button class="btn-stop-cancel" onclick={() => showStopConfirm = false}>Cancel</button>
          </div>
        </div>
      </div>
    {/if}

    {#if showExportPopup}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="dialog-overlay" onclick={() => showExportPopup = false}>
        <div class="dialog" onclick={(e) => e.stopPropagation()}>
          {#if exportDone}
            <div class="export-done">{exportDone}</div>
          {:else}
            <div class="export-title">Export Today</div>
            <div class="export-actions">
              <button class="btn-export-action" onclick={copyToClipboard}>
                <span class="export-icon">&#128203;</span> Copy to Clipboard
              </button>
              <button class="btn-export-action" onclick={downloadCsv}>
                <span class="export-icon">&#128196;</span> Download CSV
              </button>
            </div>
            <button class="btn-export-cancel" onclick={() => showExportPopup = false}>Cancel</button>
          {/if}
        </div>
      </div>
    {/if}
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

  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .dialog {
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 20px;
    width: 280px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
  }

  .dialog-msg {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin-bottom: 16px;
    text-align: center;
  }

  .dialog-msg :global(strong) {
    color: var(--accent-orange);
    font-family: var(--font-mono);
    font-weight: 600;
  }

  .dialog-btns {
    display: flex;
    gap: 8px;
  }

  .btn-stop-log {
    flex: 1;
    background: var(--accent-orange);
    border: none;
    border-radius: var(--radius-sm);
    padding: 10px;
    font-size: 13px;
    font-weight: 600;
    color: #0d0f13;
    cursor: pointer;
    font-family: var(--font-body);
  }

  .btn-stop-log:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-stop-cancel {
    flex: 1;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    font-family: var(--font-body);
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

  .export-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    text-align: center;
    margin-bottom: 14px;
  }

  .export-actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 10px;
  }

  .btn-export-action {
    width: 100%;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 12px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    cursor: pointer;
    font-family: var(--font-body);
    display: flex;
    align-items: center;
    gap: 8px;
    transition: background 0.15s;
  }

  .btn-export-action:hover {
    background: var(--bg-card-hover);
  }

  .export-icon {
    font-size: 16px;
  }

  .btn-export-cancel {
    width: 100%;
    background: transparent;
    border: none;
    padding: 8px;
    font-size: 12px;
    color: var(--text-muted);
    cursor: pointer;
    font-family: var(--font-body);
  }

  .btn-export-cancel:hover {
    color: var(--text-secondary);
  }

  .export-done {
    font-size: 14px;
    font-weight: 600;
    color: var(--accent-green);
    text-align: center;
    padding: 12px 0;
  }

  .empty-today {
    text-align: center;
    padding: 24px 16px;
  }

  .empty-today-icon {
    font-size: 24px;
    opacity: 0.4;
    margin-bottom: 6px;
  }

  .empty-today-text {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 4px;
    font-family: var(--font-body);
  }

  .empty-today-hint {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.5;
    font-family: var(--font-body);
  }
</style>
