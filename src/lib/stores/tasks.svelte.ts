import { fetchMyTasks, searchTask, pinTask as apiPinTask, unpinTask as apiUnpinTask } from '$lib/api/tauri';
import type { Task } from '$lib/types';

let tasks = $state<Task[]>([]);
let searchResults = $state<Task[]>([]);
let searchQuery = $state('');
let activeProjectFilter = $state<string | null>(null);
let searchVersion = 0;
let loading = $state(false);
let searchLoading = $state(false);
let error = $state('');

export function getTasks() { return tasks; }
export function getSearchQuery() { return searchQuery; }
export function getLoading() { return loading; }
export function getSearchLoading() { return searchLoading; }
export function getError() { return error; }

export function getPinnedTasks(): Task[] {
  return tasks.filter(t => t.pinned);
}

export function getActiveProjectFilter() { return activeProjectFilter; }

export function setActiveProjectFilter(key: string | null) {
  if (activeProjectFilter === key) return;
  activeProjectFilter = key;
  // Invalidate current in-flight requests when scope changes.
  searchVersion++;
  if (!searchQuery.trim()) {
    searchResults = [];
    return;
  }
  void search(searchQuery);
}

export function getProjectKeys(): string[] {
  // Project chips come only from "my cards" currently in the main task list.
  const keys = new Set(tasks.map(t => t.projectKey).filter(Boolean));
  return [...keys].sort();
}

export function getFilteredTasks(): Task[] {
  let result = tasks;
  if (activeProjectFilter) {
    result = result.filter(t => t.projectKey === activeProjectFilter);
  }
  if (searchQuery.trim()) {
    const q = searchQuery.toLowerCase();
    result = result.filter(t =>
      t.id.toLowerCase().includes(q) ||
      t.summary.toLowerCase().includes(q)
    );
  }
  return result;
}

export function getSearchResults(): Task[] {
  if (!activeProjectFilter) return searchResults;
  return searchResults.filter(t => t.projectKey === activeProjectFilter);
}

export function setSearchQuery(q: string) {
  searchQuery = q;
  // Invalidate any in-flight search responses when the query changes.
  searchVersion++;
  if (!q.trim()) {
    searchResults = [];
    searchLoading = false;
    return;
  }
  searchResults = [];
  // Show loading immediately so UI doesn't flash "No results" during debounce/network.
  searchLoading = true;
}

export async function refresh() {
  loading = true;
  error = '';
  try {
    tasks = await fetchMyTasks();
    // Auto-reset project filter if filtered project no longer has tasks
    if (activeProjectFilter && !tasks.some(t => t.projectKey === activeProjectFilter)) {
      activeProjectFilter = null;
    }
  } catch (e: any) {
    const msg = typeof e === 'string' ? e : e?.message || 'Failed to fetch tasks';
    error = msg;
    console.error('Failed to fetch tasks:', e);
  } finally {
    loading = false;
  }
}

export async function search(query: string) {
  searchQuery = query;
  const requestVersion = ++searchVersion;
  if (!query.trim()) {
    searchResults = [];
    searchLoading = false;
    return;
  }
  searchLoading = true;
  try {
    const results = await searchTask(query, activeProjectFilter);
    if (requestVersion !== searchVersion) return;
    const myTaskIds = new Set(tasks.map(t => t.id));
    // Results already in "my tasks" stay there; others go to searchResults
    searchResults = results
      .filter(t => !myTaskIds.has(t.id))
      .filter(t => !activeProjectFilter || t.projectKey === activeProjectFilter);
  } catch (e) {
    if (requestVersion !== searchVersion) return;
    console.error('Search failed:', e);
  } finally {
    if (requestVersion === searchVersion) {
      searchLoading = false;
    }
  }
}

export async function togglePin(taskId: string) {
  const task = tasks.find(t => t.id === taskId);
  if (!task) return;
  try {
    if (task.pinned) {
      await apiUnpinTask(taskId);
    } else {
      await apiPinTask(taskId);
    }
    tasks = tasks.map(t => t.id === taskId ? { ...t, pinned: !t.pinned } : t);
  } catch (e) {
    console.error('Toggle pin failed:', e);
  }
}
