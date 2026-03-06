import { fetchMyTasks, searchTask, pinTask as apiPinTask, unpinTask as apiUnpinTask } from '$lib/api/tauri';
import type { Task } from '$lib/types';

let tasks = $state<Task[]>([]);
let searchResults = $state<Task[]>([]);
let searchQuery = $state('');
let activeProjectFilter = $state<string | null>(null);
let loading = $state(false);
let error = $state('');

export function getTasks() { return tasks; }
export function getSearchQuery() { return searchQuery; }
export function getLoading() { return loading; }
export function getError() { return error; }

export function getPinnedTasks(): Task[] {
  return tasks.filter(t => t.pinned);
}

export function getActiveProjectFilter() { return activeProjectFilter; }

export function setActiveProjectFilter(key: string | null) {
  activeProjectFilter = key;
}

export function getProjectKeys(): string[] {
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
  return searchResults;
}

export function setSearchQuery(q: string) {
  searchQuery = q;
  if (!q.trim()) searchResults = [];
}

export async function refresh() {
  loading = true;
  error = '';
  try {
    tasks = await fetchMyTasks();
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
  if (!query.trim()) {
    searchResults = [];
    return;
  }
  try {
    const results = await searchTask(query);
    const myTaskIds = new Set(tasks.map(t => t.id));
    // Results already in "my tasks" stay there; others go to searchResults
    searchResults = results.filter(t => !myTaskIds.has(t.id));
  } catch (e) {
    console.error('Search failed:', e);
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
