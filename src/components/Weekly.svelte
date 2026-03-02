<script lang="ts">
  import { onMount } from 'svelte';
  import { getTasks } from '$lib/stores/tasks.svelte';
  import { getElapsedSecs, getTaskId, getStatus } from '$lib/stores/timer.svelte';
  import { getAggregatedEntries, getTotalSecs, refreshRange } from '$lib/stores/entriesWeekly.svelte';
  import type { WeeklyAggregatedEntry } from '$lib/stores/entriesWeekly.svelte';
  import { formatDuration, formatDurationShort } from '$lib/utils/time';

  let showExportPopup = $state(false);
  let exportDone = $state<string | null>(null);

  const colors = ['var(--accent-blue)', 'var(--accent-purple)', 'var(--accent-orange)', 'var(--accent-green)', 'var(--accent-red)'];

  const weekInfo = getCurrentWeekInfo(new Date());
  let weekStartDate = $state(weekInfo.startDate);
  let weekEndDate = $state(weekInfo.endDate);
  let weekHeader = $state(weekInfo.label);

  onMount(() => {
    refreshRange(weekStartDate, weekEndDate);
  });

  let aggEntries = $derived(getAggregatedEntries());
  let baseTotalSecs = $derived(getTotalSecs());
  let tasks = $derived(getTasks());
  let timerTaskId = $derived(getTaskId());
  let timerStatus = $derived(getStatus());
  let liveElapsed = $derived(timerStatus !== 'idle' ? getElapsedSecs() : 0);
  let totalSecs = $derived(baseTotalSecs + liveElapsed);
  let isCurrentWeek = $derived(weekStartDate === getCurrentWeekInfo(new Date()).startDate);

  function entryDuration(entry: WeeklyAggregatedEntry): number {
    if (entry.isRunning && timerStatus !== 'idle' && timerTaskId === entry.taskId) {
      return entry.totalSecs + liveElapsed;
    }
    return entry.totalSecs;
  }

  function taskName(taskId: string): string {
    return tasks.find(t => t.id === taskId)?.summary ?? taskId;
  }

  function toDateKey(date: Date): string {
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, '0');
    const d = String(date.getDate()).padStart(2, '0');
    return `${y}-${m}-${d}`;
  }

  function formatRangeLabel(start: Date, end: Date): string {
    const fmt = new Intl.DateTimeFormat([], { day: '2-digit', month: 'short' });
    const year = end.getFullYear();
    return `WEEK: ${fmt.format(start)} - ${fmt.format(end)} ${year}`;
  }

  function getCurrentWeekInfo(now: Date): { startDate: string; endDate: string; label: string } {
    const localNow = new Date(now);
    const mondayOffset = (localNow.getDay() + 6) % 7;

    const start = new Date(localNow);
    start.setDate(localNow.getDate() - mondayOffset);
    start.setHours(0, 0, 0, 0);

    const end = new Date(start);
    end.setDate(start.getDate() + 6);
    end.setHours(23, 59, 59, 999);

    return {
      startDate: toDateKey(start),
      endDate: toDateKey(end),
      label: formatRangeLabel(start, end),
    };
  }

  function parseDateKey(key: string): Date {
    const [year, month, day] = key.split('-').map(Number);
    const d = new Date(year, month - 1, day);
    d.setHours(0, 0, 0, 0);
    return d;
  }

  function setWeekFromStart(startDate: Date) {
    const start = new Date(startDate);
    start.setHours(0, 0, 0, 0);

    const end = new Date(start);
    end.setDate(start.getDate() + 6);
    end.setHours(23, 59, 59, 999);

    weekStartDate = toDateKey(start);
    weekEndDate = toDateKey(end);
    weekHeader = formatRangeLabel(start, end);

    refreshRange(weekStartDate, weekEndDate);
  }

  function goToPreviousWeek() {
    const start = parseDateKey(weekStartDate);
    start.setDate(start.getDate() - 7);
    setWeekFromStart(start);
  }

  function goToNextWeek() {
    const start = parseDateKey(weekStartDate);
    start.setDate(start.getDate() + 7);
    setWeekFromStart(start);
  }

  function goToCurrentWeek() {
    const current = getCurrentWeekInfo(new Date());
    setWeekFromStart(parseDateKey(current.startDate));
  }

  function csvEscape(value: string): string {
    if (value.includes(',') || value.includes('"') || value.includes('\n')) {
      return '"' + value.replace(/"/g, '""') + '"';
    }
    return value;
  }

  function buildExportRows(): { headers: string[]; rows: string[][] } {
    const headers = ['Task ID', 'Summary', 'Duration'];
    const rows = aggEntries.map(entry => {
      const summary = tasks.find(t => t.id === entry.taskId)?.summary ?? '';
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
      `Period: ${weekHeader}`,
      `Generated: ${generatedAt}`,
      `Total tracked: ${formatDurationShort(totalSecs)}`,
      '',
      'Task breakdown:',
    ];

    if (aggEntries.length === 0) {
      lines.push('- No tracked entries in this week.');
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
    const a = document.createElement('a');
    a.href = url;
    a.download = `ct-export-week-${weekStartDate}_to_${weekEndDate}.csv`;
    a.click();
    URL.revokeObjectURL(url);
    flashExportDone('Downloaded!');
  }
</script>

<div class="weekly">
  <div class="weekly-header">
    <div class="date-label">{weekHeader}</div>
    <div class="week-nav">
      <button class="week-btn" onclick={goToPreviousWeek}>&larr; Prev</button>
      <button class="week-btn" onclick={goToCurrentWeek} disabled={isCurrentWeek}>This Week</button>
      <button class="week-btn" onclick={goToNextWeek}>Next &rarr;</button>
    </div>
    <div class="total-row">
      <span class="total-time">{formatDurationShort(totalSecs)}</span>
      <span class="total-label">tracked this week</span>
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

  <div class="weekly-footer">
    <button class="btn-export" onclick={() => showExportPopup = true}>Export</button>
  </div>

  {#if showExportPopup}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dialog-overlay" onclick={() => showExportPopup = false}>
      <div class="dialog" onclick={(e) => e.stopPropagation()}>
        {#if exportDone}
          <div class="export-done">{exportDone}</div>
        {:else}
          <div class="export-title">Export Week</div>
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

<style>
  .weekly {
    display: flex;
    flex-direction: column;
  }

  .weekly-header {
    padding: 16px 18px 10px;
  }

  .date-label {
    font-size: 12px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    margin-bottom: 8px;
  }

  .week-nav {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 10px;
  }

  .week-btn {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }

  .week-btn:hover:enabled {
    border-color: var(--accent-blue);
    color: var(--accent-blue);
  }

  .week-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
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

  .weekly-footer {
    padding: 4px 14px 16px;
    display: flex;
    justify-content: flex-end;
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
</style>
