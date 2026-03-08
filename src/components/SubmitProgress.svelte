<script lang="ts">
  import { formatDurationShort } from '$lib/utils/time';
  import type { WorklogProgress } from '$lib/types';

  interface SubmitTask {
    taskId: string;
    totalSecs: number;
  }

  interface Props {
    tasks: SubmitTask[];
    results: WorklogProgress[];
  }

  let { tasks, results }: Props = $props();

  function getStatus(taskId: string): WorklogProgress['status'] {
    // Use the latest event for a task, not the first one.
    for (let i = results.length - 1; i >= 0; i--) {
      if (results[i].task_id === taskId) return results[i].status;
    }
    return 'pending';
  }

  function statusIcon(status: WorklogProgress['status']): string {
    switch (status) {
      case 'done': return '&#10003;';
      case 'submitting': return '&#9679;';
      case 'error': return '&#10007;';
      default: return '&#9675;';
    }
  }
</script>

<div class="submit">
  <header class="p-header">
    <div>
      <div class="p-title">Logging to Jira...</div>
      <div class="p-sub">PLEASE WAIT</div>
    </div>
  </header>

  <div class="submit-body">
    <div class="spinner"></div>
    <div class="submit-text">Submitting worklogs...</div>
    <div class="prog-list">
      {#each tasks as task (task.taskId)}
        {@const status = getStatus(task.taskId)}
        <div class="prog-item">
          <span class="prog-icon {status}">{@html statusIcon(status)}</span>
          <div class="prog-info">
            <div class="prog-key">{task.taskId}</div>
          </div>
          <span class="prog-dur">{formatDurationShort(task.totalSecs)}</span>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .submit {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .p-header {
    padding: 16px 18px 12px;
    border-bottom: 1px solid var(--border);
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

  .submit-body {
    padding: 28px 18px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
  }

  .spinner {
    width: 44px;
    height: 44px;
    border: 3px solid var(--border);
    border-top-color: var(--accent-green);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .submit-text {
    font-size: 15px;
    font-weight: 600;
    text-align: center;
  }

  .prog-list {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .prog-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: var(--bg-card);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
  }

  .prog-icon {
    font-size: 14px;
    width: 18px;
    text-align: center;
    flex-shrink: 0;
  }

  .prog-icon.done { color: var(--accent-green); }
  .prog-icon.submitting { color: var(--accent-orange); animation: blink 1s ease-in-out infinite; }
  .prog-icon.pending { color: var(--text-muted); }
  .prog-icon.error { color: var(--accent-red); }

  .prog-info {
    flex: 1;
    min-width: 0;
  }

  .prog-key {
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--text-secondary);
  }

  .prog-dur {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
</style>
