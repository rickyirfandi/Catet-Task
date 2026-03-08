import { invoke } from '@tauri-apps/api/core';
import type { JiraUser, Task, TaskDetailData, TimeEntry, TimerState, WorklogProgress } from '$lib/types';

// NOTE: Tauri v2 auto-converts camelCase (JS) → snake_case (Rust) for top-level command args.
// So JS { taskId } maps to Rust task_id. Always use camelCase here.

// ── Auth ──
export const jiraLogin = (domain: string, email: string, token: string) =>
  invoke<JiraUser>('jira_login', { domain, email, token });

export const jiraLogout = () =>
  invoke<void>('jira_logout');

export const jiraVerify = () =>
  invoke<JiraUser>('jira_verify');

export const getCurrentUser = () =>
  invoke<JiraUser | null>('get_current_user');

// ── Tasks ──
export const fetchMyTasks = () =>
  invoke<Task[]>('fetch_my_tasks');

export const searchTask = (query: string, projectKey: string | null = null) =>
  invoke<Task[]>('search_task', { query, projectKey });

export const pinTask = (taskId: string) =>
  invoke<void>('pin_task', { taskId });

export const unpinTask = (taskId: string) =>
  invoke<void>('unpin_task', { taskId });

export const getTaskDetail = (taskId: string) =>
  invoke<TaskDetailData>('get_task_detail', { taskId });

// ── Timer ──
export const startTimer = (taskId: string) =>
  invoke<void>('start_timer', { taskId });

export const stopTimer = () =>
  invoke<void>('stop_timer');

export const pauseTimer = () =>
  invoke<void>('pause_timer');

export const resumeTimer = () =>
  invoke<void>('resume_timer');

export const getActiveTimer = () =>
  invoke<TimerState>('get_active_timer');

// ── Entries ──
export const getEntriesToday = () =>
  invoke<TimeEntry[]>('get_entries_today');

export const getEntriesRange = (startDate: string, endDate: string) =>
  invoke<TimeEntry[]>('get_entries_range', { startDate, endDate });

export const updateEntry = (
  entryId: number,
  adjustedSecs: number | null,
  description: string | null,
  date: string | null,
  startedAt: string | null = null,
) =>
  invoke<void>('update_entry', { entryId, adjustedSecs, description, date, startedAt });

// ── Worklog ──
export const submitBatchWorklog = (entries: { entryIds: number[]; taskId: string; timeSpentSeconds: number; started: string; comment: string }[]) =>
  invoke<void>('submit_batch_worklog', { entries });

// ── App ──
export const quitApp = () =>
  invoke<void>('quit_app');

export const openJira = (issueKey: string | null = null) =>
  invoke<void>('open_jira', { issueKey });

// ── Settings ──
export const getSetting = (key: string) =>
  invoke<string | null>('get_setting', { key });

export const setSetting = (key: string, value: string) =>
  invoke<void>('set_setting', { key, value });

export const setLaunchAtLogin = (enabled: boolean) =>
  invoke<void>('set_launch_at_login', { enabled });

export const resetTimerData = () =>
  invoke<number>('reset_timer_data');
