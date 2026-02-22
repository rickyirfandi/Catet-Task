import { fetchMyTasks, searchTask, pinTask as apiPinTask, unpinTask as apiUnpinTask } from '$lib/api/tauri';
import type { Task } from '$lib/types';

let tasks = $state<Task[]>([]);
let searchQuery = $state('');
let loading = $state(false);
let error = $state('');

export function getTasks() { return tasks; }
export function getSearchQuery() { return searchQuery; }
export function getLoading() { return loading; }
export function getError() { return error; }

export function getPinnedTasks(): Task[] {
  return tasks.filter(t => t.pinned);
}

export function getFilteredTasks(): Task[] {
  if (!searchQuery.trim()) return tasks;
  const q = searchQuery.toLowerCase();
  return tasks.filter(t =>
    t.id.toLowerCase().includes(q) ||
    t.summary.toLowerCase().includes(q)
  );
}

export function setSearchQuery(q: string) {
  searchQuery = q;
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
  if (!query.trim()) return;
  try {
    const results = await searchTask(query);
    // Merge results with existing tasks, avoiding duplicates
    const existing = new Set(tasks.map(t => t.id));
    for (const task of results) {
      if (!existing.has(task.id)) {
        tasks = [...tasks, task];
      }
    }
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
