export interface JiraUser {
  id: string;
  email: string;
  displayName: string;
  avatarUrl: string;
  jiraDomain: string;
  authMethod: string;
}

export interface Task {
  id: string;
  summary: string;
  projectKey: string;
  projectName: string;
  status: string;
  sprintName: string | null;
  pinned: boolean;
  lastFetched: string | null;
  inCurrentSprint: boolean;
  parentKey: string | null;
  parentSummary: string | null;
}

export interface TaskDetailData {
  taskId: string;
  summary: string;
  description: string | null;
  status: string;
  projectKey: string;
  projectName: string;
  parentKey: string | null;
  parentSummary: string | null;
  issueType: string | null;
  priority: string | null;
  assignee: string | null;
  updatedAt: string | null;
  createdAt: string | null;
}

export interface TimeEntry {
  id: number;
  taskId: string;
  startTime: string;
  endTime: string | null;
  durationSecs: number | null;
  adjustedSecs: number | null;
  description: string | null;
  syncedToJira: boolean;
  jiraWorklogId: string | null;
}

export type TimerStatus = 'idle' | 'running' | 'paused';

export interface TimerState {
  status: TimerStatus;
  taskId: string | null;
  elapsedSecs: number;
}

export interface TimerTickPayload {
  status: TimerStatus;
  task_id: string | null;
  elapsed_secs: number;
}

export interface WorklogSubmission {
  entryIds: number[];
  taskId: string;
  timeSpentSeconds: number;
  started: string;
  comment: string;
}

export interface WorklogProgress {
  task_id: string;
  status: 'pending' | 'submitting' | 'done' | 'error';
  error: string | null;
  worklog_id: string | null;
}

export type AppView = 'login' | 'main';
export type TabId = 'timer' | 'today' | 'weekly' | 'settings';
export type LogFlowStep = 'select' | 'edit' | 'submitting' | 'result';
