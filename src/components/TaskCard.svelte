<script lang="ts">
  import Badge from './shared/Badge.svelte';
  import { getTaskId, getElapsedSecs, getStatus } from '$lib/stores/timer.svelte';
  import { toggle } from '$lib/stores/timer.svelte';
  import { getTaskTotalSecs } from '$lib/stores/entries.svelte';
  import { formatDuration, formatDurationShort } from '$lib/utils/time';
  import type { Task, TimeEntry } from '$lib/types';

  interface Props {
    task: Task;
    entry?: TimeEntry | null;
    logged?: boolean;
    onSelect?: ((task: Task) => void) | null;
  }

  let { task, entry = null, logged = false, onSelect = null }: Props = $props();

  let isActive = $derived(getTaskId() === task.id && getStatus() !== 'idle');
  let isTimerRunning = $derived(getTaskId() === task.id && getStatus() === 'running');
  let isPaused = $derived(getTaskId() === task.id && getStatus() === 'paused');

  // Show total time tracked today for this task (all sessions combined + live elapsed)
  let pastSecs = $derived(getTaskTotalSecs(task.id));
  let displayTime = $derived.by(() => {
    if (isActive) {
      const total = pastSecs + getElapsedSecs();
      return formatDuration(total);
    }
    if (pastSecs > 0) return formatDurationShort(pastSecs);
    return '--:--';
  });

  let statusBadge = $derived.by(() => {
    if (isTimerRunning) return 'running';
    if (isPaused) return 'stopped';
    const s = task.status?.toLowerCase() ?? '';
    if (s.includes('progress')) return 'progress';
    if (s.includes('review')) return 'review';
    if (s.includes('done')) return 'logged';
    return 'todo';
  });

  function handlePlay(e: MouseEvent) {
    e.stopPropagation();
    toggle(task.id);
  }

  function handleCardClick() {
    if (logged) return;
    if (onSelect) {
      onSelect(task);
    } else {
      toggle(task.id);
    }
  }
</script>

{#if logged}
  <div class="task logged">
    <div class="task-top">
      <div class="logged-row">
        <span class="task-key muted">{task.id}</span>
        <span class="logged-name">{task.summary}</span>
        <span class="logged-dur">{entry ? formatDurationShort(entry.adjustedSecs ?? entry.durationSecs ?? 0) : ''} &#10003;</span>
      </div>
    </div>
  </div>
{:else}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="task" class:active={isActive} onclick={handleCardClick}>
    <div class="task-top">
      <span class="task-key">{task.id}</span>
      <Badge variant={statusBadge} />
    </div>
    <div class="task-name">{task.summary}</div>
    <div class="task-bottom">
      <span class="task-project">{task.projectKey}{task.sprintName ? ` · ${task.sprintName}` : ''}{#if task.inCurrentSprint && !task.sprintName}<span class="sprint-tag">Sprint</span>{/if}</span>
      <div class="task-timer">
        <span class="task-time">{displayTime}</span>
        <button class="play-btn" class:on={isActive} onclick={handlePlay}>
          {isTimerRunning ? '⏸' : '▶'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .task {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 11px 13px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .task:hover {
    background: var(--bg-card-hover);
  }

  .task.active {
    background: var(--bg-card-active);
    border-color: var(--accent-blue);
    box-shadow: 0 0 0 1px var(--accent-blue), 0 4px 16px rgba(61, 122, 237, 0.1);
  }

  .task-top {
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

  .task.active .task-key {
    color: var(--accent-green);
  }

  .task-key.muted {
    color: var(--text-muted);
  }

  .task-name {
    font-size: 13px;
    font-weight: 500;
    line-height: 1.35;
    margin-bottom: 7px;
  }

  .task-bottom {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .task-project {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .sprint-tag {
    margin-left: 6px;
    font-size: 9px;
    font-weight: 600;
    color: var(--accent-purple);
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }

  .task-timer {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .task-time {
    font-size: 13px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--text-muted);
  }

  .task.active .task-time {
    color: var(--accent-green);
  }

  .play-btn {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 10px;
    background: var(--bg-card-hover);
    color: var(--text-secondary);
    transition: all 0.15s;
  }

  .play-btn.on {
    background: var(--accent-green);
    color: #0d0f13;
  }

  /* Logged state */
  .task.logged {
    opacity: 0.4;
    padding: 8px 13px;
  }

  .task.logged .task-top {
    margin-bottom: 0;
  }

  .logged-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
  }

  .logged-name {
    font-size: 11px;
    color: var(--text-muted);
    margin-left: 8px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .logged-dur {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    flex-shrink: 0;
  }
</style>
