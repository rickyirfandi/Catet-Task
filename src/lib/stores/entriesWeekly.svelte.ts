import { getEntriesRange } from '$lib/api/tauri';
import type { TimeEntry } from '$lib/types';

export interface WeeklyAggregatedEntry {
  taskId: string;
  entryIds: number[];
  totalSecs: number;
  isRunning: boolean;
  isSynced: boolean;
  latestStartTime: string;
}

let entries = $state<TimeEntry[]>([]);
let currentStartDate = $state<string | null>(null);
let currentEndDate = $state<string | null>(null);

export function getEntries() { return entries; }
export function getCurrentStartDate() { return currentStartDate; }
export function getCurrentEndDate() { return currentEndDate; }

export function getTotalSecs(): number {
  return entries.reduce((sum, e) => sum + (e.adjustedSecs ?? e.durationSecs ?? 0), 0);
}

export function getAggregatedEntries(): WeeklyAggregatedEntry[] {
  const map = new Map<string, WeeklyAggregatedEntry>();
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

  return [...map.values()].sort((a, b) => {
    if (a.isRunning !== b.isRunning) return a.isRunning ? -1 : 1;
    return b.latestStartTime.localeCompare(a.latestStartTime);
  });
}

export async function refreshRange(startDate: string, endDate: string) {
  try {
    currentStartDate = startDate;
    currentEndDate = endDate;
    entries = await getEntriesRange(startDate, endDate);
  } catch (e) {
    console.error('Failed to fetch weekly entries:', e);
  }
}
