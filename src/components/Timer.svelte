<script lang="ts">
  import SearchBar from './SearchBar.svelte';
  import TaskCard from './TaskCard.svelte';
  import TaskDetail from './TaskDetail.svelte';
  import { getFilteredTasks, getSearchResults, getSearchQuery, getLoading, getSearchLoading, getError, refresh as refreshTasks } from '$lib/stores/tasks.svelte';
  import { getTaskId, getStatus } from '$lib/stores/timer.svelte';
  import { getEntries, getLoggedEntries, getUnloggedEntries, getTaskTotalSecs } from '$lib/stores/entries.svelte';
  import { refresh as refreshEntries } from '$lib/stores/entries.svelte';
  import { onMount } from 'svelte';
  import type { Task } from '$lib/types';

  onMount(() => {
    refreshEntries();
  });

  let tasks = $derived(getFilteredTasks());
  let searchResultTasks = $derived(getSearchResults());
  let searchQuery = $derived(getSearchQuery());
  let isLoading = $derived(getLoading());
  let isSearchLoading = $derived(getSearchLoading());
  let fetchError = $derived(getError());
  let activeTaskId = $derived(getTaskId());
  let timerStatus = $derived(getStatus());
  let entries = $derived(getEntries());
  let logged = $derived(getLoggedEntries());
  let unlogged = $derived(getUnloggedEntries());

  let activeTask = $derived(tasks.find(t => t.id === activeTaskId));

  // Set of task IDs that have been logged to Jira
  let loggedTaskIds = $derived(new Set(logged.map(e => e.taskId)));

  // Pinned tasks surfaced at top (excluding active and logged), scoped to current project filter.
  let pinnedTasks = $derived(tasks.filter(t => t.pinned && t.id !== activeTaskId && !loggedTaskIds.has(t.id)));
  let pinnedTaskIds = $derived(new Set(pinnedTasks.map(t => t.id)));

  // Set of task IDs that have unlogged time tracked (stopped entries with duration > 0)
  let unloggedTaskIds = $derived(new Set(
    entries
      .filter(e => !e.syncedToJira && e.endTime !== null && (e.durationSecs ?? 0) > 0)
      .map(e => e.taskId)
  ));

  // Tasks with unlogged tracked time (excluding currently active task and pinned)
  let unloggedTasks = $derived(
    tasks.filter(t => t.id !== activeTaskId && unloggedTaskIds.has(t.id) && !loggedTaskIds.has(t.id) && !pinnedTaskIds.has(t.id))
  );

  // Tasks with no tracked time today (excluding pinned)
  let freshTasks = $derived(
    tasks.filter(t => t.id !== activeTaskId && !unloggedTaskIds.has(t.id) && !loggedTaskIds.has(t.id) && !pinnedTaskIds.has(t.id))
  );

  // Whether to show empty state (no tasks at all, not loading, no error)
  let showEmpty = $derived(!isLoading && !fetchError && tasks.length === 0 && !searchQuery);

  function entryForTask(taskId: string) {
    return entries.find(e => e.taskId === taskId);
  }

  let selectedTaskId = $state<string | null>(null);
  // Always derived from the live tasks store — reflects pin changes, status updates, etc.
  let selectedTask = $derived(selectedTaskId
    ? (tasks.find(t => t.id === selectedTaskId) ?? searchResultTasks.find(t => t.id === selectedTaskId) ?? null)
    : null);
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

  {#if pinnedTasks.length > 0}
    <div class="group-label pinned">&#9733; Pinned</div>
    {#each pinnedTasks as task (task.id)}
      <TaskCard {task} entry={entryForTask(task.id)} onSelect={(t) => selectedTaskId = t.id} />
    {/each}
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

  {#if searchQuery && searchResultTasks.length > 0}
    <div class="group-label purple">Search Results</div>
    {#each searchResultTasks as task (task.id)}
      <TaskCard {task} onSelect={(t) => selectedTaskId = t.id} />
    {/each}
  {:else if searchQuery && isSearchLoading}
    <div class="status-msg searching">
      <span class="spinner" aria-hidden="true"></span>
      <span>Searching Jira...</span>
    </div>
  {:else if searchQuery && searchResultTasks.length === 0 && !isLoading}
    <div class="empty-state">No results for "{searchQuery}"</div>
  {/if}

  {#if showEmpty}
    <div class="empty-state">
      <div class="empty-icon">&#128203;</div>
      <div class="empty-title">No tasks yet</div>
      <div class="empty-desc">Search for a Jira task or fetch your assigned tasks to get started.</div>
      <button class="empty-btn" onclick={() => refreshTasks()}>Fetch My Tasks</button>
    </div>
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

  .group-label.purple {
    color: var(--accent-purple);
  }

  .status-msg {
    font-size: 12px;
    color: var(--text-muted);
    text-align: center;
    padding: 16px;
    font-family: var(--font-mono);
  }

  .status-msg.searching {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .spinner {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid rgba(61, 122, 237, 0.25);
    border-top-color: var(--accent-blue);
    animation: spin 0.75s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
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

  .group-label.pinned {
    color: var(--accent-orange);
  }

  .empty-state {
    text-align: center;
    padding: 28px 16px;
    color: var(--text-muted);
    font-size: 12px;
    font-family: var(--font-mono);
  }

  .empty-icon {
    font-size: 28px;
    margin-bottom: 8px;
    opacity: 0.5;
  }

  .empty-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 4px;
    font-family: var(--font-body);
  }

  .empty-desc {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.5;
    margin-bottom: 12px;
    font-family: var(--font-body);
  }

  .empty-btn {
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    padding: 6px 14px;
    border-radius: 4px;
    background: rgba(61, 122, 237, 0.1);
    border: 1px solid rgba(61, 122, 237, 0.3);
    color: var(--accent-blue);
    cursor: pointer;
  }

  .empty-btn:hover {
    background: rgba(61, 122, 237, 0.2);
  }
</style>
