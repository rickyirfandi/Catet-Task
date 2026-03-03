<script lang="ts">
  import { formatDurationShort } from '$lib/utils/time';
  import { getAggregatedEntries } from '$lib/stores/entries.svelte';
  import { openJira } from '$lib/api/tauri';
  import type { WorklogProgress } from '$lib/types';

  interface SubmitTask {
    taskId: string;
    totalSecs: number;
  }

  interface Props {
    tasks: SubmitTask[];
    results: WorklogProgress[];
    onclose: () => void;
  }

  let { tasks, results, onclose }: Props = $props();

  let logged = $derived(results.filter(r => r.status === 'done'));
  let failed = $derived(results.filter(r => r.status === 'error'));
  let hasErrors = $derived(failed.length > 0);
  let openingJira = $state(false);

  // Tasks still tracking (running or not submitted)
  let submittedTaskIds = $derived(new Set(results.map(r => r.task_id)));
  let stillTracking = $derived(getAggregatedEntries().filter(e =>
    e.isRunning || (!e.isSynced && !submittedTaskIds.has(e.taskId))
  ));

  function taskDuration(taskId: string): number {
    return tasks.find(t => t.taskId === taskId)?.totalSecs ?? 0;
  }

  function firstLoggedTaskId(): string | null {
    if (logged.length === 0) return null;
    return logged[0].task_id;
  }

  async function handleOpenJira() {
    if (openingJira) return;
    openingJira = true;
    try {
      await openJira(firstLoggedTaskId());
    } catch (e) {
      console.error('Failed to open Jira:', e);
    } finally {
      openingJira = false;
    }
  }
</script>

<div class="success">
  <div class="success-body">
    {#if hasErrors}
      <div class="success-icon warn">&#9888;</div>
      <div class="success-title">Partially Logged</div>
      <div class="success-detail">
        {logged.length} of {results.length} worklogs submitted<br />
        {failed.length} entry failed — you can retry
      </div>
    {:else}
      <div class="success-icon" aria-hidden="true">
        <svg class="success-check" viewBox="0 0 24 24" fill="none">
          <path d="M20 6.5L9 17.5L4 12.5" />
        </svg>
      </div>
      <div class="success-title">Logged!</div>
      <div class="success-detail">
        {logged.length} worklogs submitted to Jira
        {#if stillTracking.length > 0}
          <br />{stillTracking.length} tasks kept for continued tracking
        {/if}
      </div>
    {/if}

    <div class="result-list">
      {#if logged.length > 0}
        <div class="res-group">&#10003; Logged to Jira</div>
        {#each logged as r}
          <div class="res-item">
            <div class="res-left">
              <span class="res-icon done">&#10003;</span>
              <span class="res-key">{r.task_id}</span>
            </div>
            <span class="res-dur">{formatDurationShort(taskDuration(r.task_id))}</span>
            <span class="res-status logged">logged</span>
          </div>
        {/each}
      {/if}

      {#if failed.length > 0}
        <div class="res-group">&#10007; Failed</div>
        {#each failed as r}
          <div class="res-item error">
            <div class="res-left">
              <span class="res-icon err">&#10007;</span>
              <span class="res-key">{r.task_id}</span>
            </div>
            <span class="res-status err-text">{r.error ?? 'error'}</span>
          </div>
        {/each}
      {/if}

      {#if stillTracking.length > 0}
        <div class="res-group">&#9203; Still Tracking</div>
        {#each stillTracking as entry}
          <div class="res-item">
            <div class="res-left">
              <span class="res-icon muted">&#9679;</span>
              <span class="res-key muted">{entry.taskId}</span>
            </div>
            <span class="res-dur muted">{formatDurationShort(entry.totalSecs)}</span>
            <span class="res-status kept">{entry.isRunning ? 'running' : 'kept'}</span>
          </div>
        {/each}
      {/if}
    </div>
  </div>

  <div class="success-footer">
    <button class="btn-done" onclick={onclose}>Close</button>
    {#if hasErrors}
      <button class="btn-jira">Retry Failed</button>
    {:else}
      <button class="btn-jira" onclick={handleOpenJira} disabled={openingJira}>
        {openingJira ? 'Opening...' : 'Open Jira &#8599;'}
      </button>
    {/if}
  </div>
</div>

<style>
  .success {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .success-body {
    padding: 28px 18px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    flex: 1;
    overflow-y: auto;
  }

  .success-icon {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: var(--accent-green-dim);
    border: 2px solid var(--accent-green);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 4px;
    animation: scaleIn 0.3s ease-out;
  }

  .success-icon.warn {
    border-color: var(--accent-orange);
    background: var(--accent-orange-dim);
    font-size: 24px;
    line-height: 1;
  }

  .success-check {
    width: 28px;
    height: 28px;
    stroke: var(--accent-green);
    stroke-width: 2.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .success-title {
    font-size: 17px;
    font-weight: 700;
  }

  .success-detail {
    font-size: 13px;
    color: var(--text-secondary);
    text-align: center;
    line-height: 1.5;
  }

  .result-list {
    width: 100%;
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .res-group {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    letter-spacing: 1.5px;
    text-transform: uppercase;
    font-family: var(--font-mono);
    margin-top: 8px;
    margin-bottom: 2px;
  }

  .res-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 9px 13px;
    background: var(--bg-card);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
  }

  .res-item.error {
    border-color: rgba(239, 87, 87, 0.3);
  }

  .res-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .res-icon {
    font-size: 13px;
    width: 16px;
    text-align: center;
  }

  .res-icon.done { color: var(--accent-green); }
  .res-icon.err { color: var(--accent-red); }
  .res-icon.muted { color: var(--text-muted); }

  .res-key {
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--accent-blue);
  }

  .res-key.muted {
    color: var(--text-muted);
  }

  .res-dur {
    font-size: 12px;
    font-weight: 600;
    font-family: var(--font-mono);
  }

  .res-dur.muted {
    color: var(--text-muted);
  }

  .res-status {
    font-size: 10px;
    font-family: var(--font-mono);
  }

  .res-status.logged { color: var(--accent-green); }
  .res-status.kept { color: var(--accent-orange); }
  .res-status.err-text { color: var(--accent-red); }

  .success-footer {
    padding: 8px 18px 18px;
    display: flex;
    gap: 8px;
  }

  .btn-done {
    flex: 1;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 11px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    cursor: pointer;
    font-family: var(--font-body);
    text-align: center;
  }

  .btn-jira {
    flex: 1;
    background: linear-gradient(135deg, var(--accent-blue), #5b8def);
    border: none;
    border-radius: var(--radius-sm);
    padding: 11px;
    font-size: 13px;
    font-weight: 600;
    color: white;
    cursor: pointer;
    font-family: var(--font-body);
    text-align: center;
  }

  .btn-jira:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }
</style>
