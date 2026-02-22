import { getEntriesToday } from '$lib/api/tauri';
import type { TimeEntry } from '$lib/types';

export interface AggregatedEntry {
  taskId: string;
  entryIds: number[];
  totalSecs: number;
  isRunning: boolean;
  isSynced: boolean;
  latestStartTime: string;
}

let entries = $state<TimeEntry[]>([]);
let selectedIds = $state<Set<number>>(new Set());

export function getEntries() { return entries; }
export function getSelectedIds() { return selectedIds; }

export function getLoggedEntries(): TimeEntry[] {
  return entries.filter(e => e.syncedToJira);
}

export function getUnloggedEntries(): TimeEntry[] {
  return entries.filter(e => !e.syncedToJira && e.endTime !== null);
}

export function getRunningEntry(): TimeEntry | undefined {
  return entries.find(e => e.endTime === null);
}

export function getTotalSecs(): number {
  return entries.reduce((sum, e) => sum + (e.adjustedSecs ?? e.durationSecs ?? 0), 0);
}

/** Aggregate entries by task for the Today view */
export function getAggregatedEntries(): AggregatedEntry[] {
  const map = new Map<string, AggregatedEntry>();
  for (const e of entries) {
    const existing = map.get(e.taskId);
    const secs = e.adjustedSecs ?? e.durationSecs ?? 0;
    if (existing) {
      existing.entryIds.push(e.id);
      existing.totalSecs += secs;
      if (e.endTime === null) existing.isRunning = true;
      if (e.syncedToJira) existing.isSynced = true;
      if (e.startTime > existing.latestStartTime) existing.latestStartTime = e.startTime;
    } else {
      map.set(e.taskId, {
        taskId: e.taskId,
        entryIds: [e.id],
        totalSecs: secs,
        isRunning: e.endTime === null,
        isSynced: e.syncedToJira,
        latestStartTime: e.startTime,
      });
    }
  }
  // Sort: running first, then by latest start time desc
  return [...map.values()].sort((a, b) => {
    if (a.isRunning !== b.isRunning) return a.isRunning ? -1 : 1;
    return b.latestStartTime.localeCompare(a.latestStartTime);
  });
}

/** Get total tracked seconds for a specific task */
export function getTaskTotalSecs(taskId: string): number {
  return entries
    .filter(e => e.taskId === taskId)
    .reduce((sum, e) => sum + (e.adjustedSecs ?? e.durationSecs ?? 0), 0);
}

export function getSelectedSecs(): number {
  return entries
    .filter(e => selectedIds.has(e.id))
    .reduce((sum, e) => sum + (e.adjustedSecs ?? e.durationSecs ?? 0), 0);
}

export function isSelected(id: number): boolean {
  return selectedIds.has(id);
}

export function toggleSelect(id: number) {
  const next = new Set(selectedIds);
  if (next.has(id)) {
    next.delete(id);
  } else {
    next.add(id);
  }
  selectedIds = next;
}

export function selectAll() {
  selectedIds = new Set(entries.map(e => e.id));
}

export function selectNone() {
  selectedIds = new Set();
}

export function selectStopped() {
  selectedIds = new Set(entries.filter(e => e.endTime !== null && !e.syncedToJira).map(e => e.id));
}

export async function refresh() {
  try {
    entries = await getEntriesToday();
    // Default: select stopped, unsynced entries
    selectStopped();
  } catch (e) {
    console.error('Failed to fetch entries:', e);
  }
}
