<script lang="ts">
  import SearchBar from './SearchBar.svelte';
  import TaskCard from './TaskCard.svelte';
  import TaskDetail from './TaskDetail.svelte';
  import { getFilteredTasks, getLoading, getError, refresh as refreshTasks } from '$lib/stores/tasks.svelte';
  import { getTaskId, getStatus } from '$lib/stores/timer.svelte';
  import { getEntries, getLoggedEntries, getUnloggedEntries, getTaskTotalSecs } from '$lib/stores/entries.svelte';
  import { refresh as refreshEntries } from '$lib/stores/entries.svelte';
  import { onMount } from 'svelte';
  import type { Task } from '$lib/types';

  onMount(() => {
    refreshEntries();
  });

  let tasks = $derived(getFilteredTasks());
  let isLoading = $derived(getLoading());
  let fetchError = $derived(getError());
  let activeTaskId = $derived(getTaskId());
  let timerStatus = $derived(getStatus());
  let entries = $derived(getEntries());
  let logged = $derived(getLoggedEntries());
  let unlogged = $derived(getUnloggedEntries());

  let activeTask = $derived(tasks.find(t => t.id === activeTaskId));

  // Set of task IDs that have been logged to Jira
  let loggedTaskIds = $derived(new Set(logged.map(e => e.taskId)));

  // Set of task IDs that have unlogged time tracked (stopped entries with duration > 0)
  let unloggedTaskIds = $derived(new Set(
    entries
      .filter(e => !e.syncedToJira && e.endTime !== null && (e.durationSecs ?? 0) > 0)
      .map(e => e.taskId)
  ));

  // Tasks with unlogged tracked time (excluding currently active task)
  let unloggedTasks = $derived(
    tasks.filter(t => t.id !== activeTaskId && unloggedTaskIds.has(t.id) && !loggedTaskIds.has(t.id))
  );

  // Tasks with no tracked time today
  let freshTasks = $derived(
    tasks.filter(t => t.id !== activeTaskId && !unloggedTaskIds.has(t.id) && !loggedTaskIds.has(t.id))
  );

  function entryForTask(taskId: string) {
    return entries.find(e => e.taskId === taskId);
  }

  let selectedTaskId = $state<string | null>(null);
  // Always derived from the live tasks store — reflects pin changes, status updates, etc.
  let selectedTask = $derived(selectedTaskId ? (tasks.find(t => t.id === selectedTaskId) ?? null) : null);
</script>

{#if selectedTask}
  <TaskDetail task={selectedTask} onBack={() => selectedTaskId = null} />
{:else}
<SearchBar />

<div class="task-list">
  {#if isLoading}
    <div class="status-msg">Loading tasks...</div>
  {/if}

  {#if fetchError}
    <div class="error-msg">
      <span>{fetchError}</span>
      <button class="retry-btn" onclick={() => refreshTasks()}>Retry</button>
    </div>
  {/if}

  {#if activeTask && timerStatus !== 'idle'}
    <div class="group-label">&#9654; Currently Tracking</div>
    <TaskCard task={activeTask} entry={entryForTask(activeTask.id)} onSelect={(t) => selectedTaskId = t.id} />
  {/if}

  {#if unloggedTasks.length > 0}
    <div class="group-label">Unlogged</div>
    {#each unloggedTasks as task (task.id)}
      <TaskCard {task} entry={entryForTask(task.id)} onSelect={(t) => selectedTaskId = t.id} />
    {/each}
  {/if}

  {#if freshTasks.length > 0}
    <div class="group-label">My Tasks</div>
    {#each freshTasks as task (task.id)}
      <TaskCard {task} entry={entryForTask(task.id)} onSelect={(t) => selectedTaskId = t.id} />
    {/each}
  {/if}

  {#if logged.length > 0}
    <div class="group-label green">&#10003; Logged Today</div>
    {#each logged as entry (entry.id)}
      {@const task = tasks.find(t => t.id === entry.taskId)}
      {#if task}
        <TaskCard {task} {entry} logged={true} />
      {/if}
    {/each}
  {/if}
</div>
{/if}

<style>
  .task-list {
    padding: 0 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .group-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    letter-spacing: 1.5px;
    text-transform: uppercase;
    font-family: var(--font-mono);
    padding: 8px 4px 4px;
  }

  .group-label.green {
    color: var(--accent-green);
  }

  .status-msg {
    font-size: 12px;
    color: var(--text-muted);
    text-align: center;
    padding: 16px;
    font-family: var(--font-mono);
  }

  .error-msg {
    font-size: 11px;
    color: var(--accent-red);
    background: rgba(239, 87, 87, 0.08);
    border: 1px solid rgba(239, 87, 87, 0.2);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .retry-btn {
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-mono);
    padding: 4px 10px;
    border-radius: 4px;
    background: rgba(239, 87, 87, 0.15);
    border: 1px solid rgba(239, 87, 87, 0.3);
    color: var(--accent-red);
    cursor: pointer;
    white-space: nowrap;
  }

  .retry-btn:hover {
    background: rgba(239, 87, 87, 0.25);
  }
</style>
