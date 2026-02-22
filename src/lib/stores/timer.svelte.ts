import { startTimer, stopTimer, pauseTimer, resumeTimer, getActiveTimer } from '$lib/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { refresh as refreshEntries } from '$lib/stores/entries.svelte';
import type { TimerStatus, TimerTickPayload } from '$lib/types';

let status = $state<TimerStatus>('idle');
let taskId = $state<string | null>(null);
let elapsedSecs = $state(0);

export function getStatus() { return status; }
export function getTaskId() { return taskId; }
export function getElapsedSecs() { return elapsedSecs; }
export function isRunning() { return status === 'running'; }
export function isPaused() { return status === 'paused'; }

let unlisten: (() => void) | null = null;

export async function init() {
  console.log('[JTT] timer init() called');
  // Fetch current state
  try {
    const state = await getActiveTimer();
    console.log('[JTT] initial timer state:', JSON.stringify(state));
    status = state.status;
    taskId = state.taskId;
    elapsedSecs = state.elapsedSecs;
  } catch (e) {
    console.error('[JTT] getActiveTimer failed:', e);
  }

  // Listen for tick events
  if (unlisten) unlisten();
  unlisten = await listen<TimerTickPayload>('timer-tick', (event) => {
    const prev = status;
    status = event.payload.status as TimerStatus;
    taskId = event.payload.task_id;
    elapsedSecs = event.payload.elapsed_secs;
    if (prev !== status || (status !== 'idle' && elapsedSecs % 10 === 0)) {
      console.log('[JTT] tick:', status, taskId, elapsedSecs);
    }
  });
  console.log('[JTT] timer event listener registered');
}

export async function toggle(id: string) {
  try {
    if (taskId === id && status === 'running') {
      await pauseTimer();
    } else if (taskId === id && status === 'paused') {
      await resumeTimer();
    } else {
      // Starting a new task stops the previous one — refresh entries to reflect finalized entry
      await startTimer(id);
      await refreshEntries();
    }
  } catch (e) {
    console.error('[JTT] Timer toggle failed:', e);
  }
}

export async function stop() {
  await stopTimer();
  await refreshEntries();
}

export function destroy() {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}
