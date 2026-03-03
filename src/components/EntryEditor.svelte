<script lang="ts">
  import DurationInput from './shared/DurationInput.svelte';
  import CommentField from './shared/CommentField.svelte';
  import { getTasks } from '$lib/stores/tasks.svelte';
  import { updateEntry } from '$lib/api/tauri';
  import {
    formatTime,
    formatDurationShort,
    getTodayLocalDateKey,
    toLocalDateKeyFromValue,
  } from '$lib/utils/time';
  import type { TimeEntry } from '$lib/types';

  interface Props {
    entry: TimeEntry;
    onback: () => void;
    onsave: () => void;
  }

  let { entry, onback, onsave }: Props = $props();

  let adjustedSecs = $state(entry.adjustedSecs ?? entry.durationSecs ?? 0);
  let description = $state(entry.description ?? '');
  let date = $state(toLocalDateKeyFromValue(entry.startTime) ?? getTodayLocalDateKey());

  let task = $derived(getTasks().find(t => t.id === entry.taskId));
  let rawSecs = $derived(entry.durationSecs ?? 0);
  let isToday = $derived(date === getTodayLocalDateKey());

  async function handleSave() {
    try {
      await updateEntry(entry.id, adjustedSecs, description || null, date);
      onsave();
    } catch (e) {
      console.error('Failed to save entry:', e);
    }
  }
</script>

<div class="editor">
  <header class="p-header">
    <div class="p-header-left">
      <button class="back-btn" onclick={onback}>&larr;</button>
      <div>
        <div class="p-title">Edit Worklog</div>
      </div>
    </div>
  </header>

  <div class="edit-task-hdr">
    <div class="edit-key">{entry.taskId}</div>
    <div class="edit-name">{task?.summary ?? entry.taskId}</div>
  </div>

  <div class="edit-form">
    <div class="field">
      <div class="field-label">
        Duration
        <span class="field-hint">tracked: {formatDurationShort(rawSecs)}</span>
      </div>
      <DurationInput totalSecs={adjustedSecs} onchange={(s) => adjustedSecs = s} />
    </div>

    <div class="field">
      <div class="field-label">Date</div>
      <div class="date-row">
        <input class="date-val" type="date" bind:value={date} />
        {#if isToday}
          <span class="date-badge">TODAY</span>
        {/if}
      </div>
    </div>

    <div class="field">
      <div class="field-label">Time Range</div>
      <div class="time-range">
        <span class="tr-val">{formatTime(entry.startTime)}</span>
        <span class="tr-sep">&rarr;</span>
        <span class="tr-val">{entry.endTime ? formatTime(entry.endTime) : 'now'}</span>
      </div>
    </div>

    <div class="field">
      <div class="field-label">
        Work Description
        <span class="field-hint">&rarr; Jira worklog comment</span>
      </div>
      <CommentField value={description} onchange={(v) => description = v} />
    </div>
  </div>

  <div class="edit-btns">
    <button class="btn-save" onclick={handleSave}>&#10003; Save Entry</button>
    <button class="btn-skip" onclick={onback}>Skip</button>
  </div>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .p-header {
    padding: 16px 18px 12px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
  }

  .p-header-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .back-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-size: 14px;
    cursor: pointer;
  }

  .p-title {
    font-size: 15px;
    font-weight: 600;
  }

  .edit-task-hdr {
    padding: 14px 18px;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
  }

  .edit-key {
    font-size: 12px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--accent-blue);
    margin-bottom: 3px;
  }

  .edit-name {
    font-size: 14px;
    font-weight: 500;
    line-height: 1.4;
  }

  .edit-form {
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    flex: 1;
    overflow-y: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .field-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.5px;
    text-transform: uppercase;
    font-family: var(--font-mono);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .field-hint {
    font-size: 10px;
    color: var(--text-muted);
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0;
  }

  .date-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .date-val {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px 14px;
    font-size: 13px;
    font-family: var(--font-mono);
    color: var(--text-primary);
    flex: 1;
  }

  .date-badge {
    font-size: 10px;
    font-weight: 600;
    color: var(--accent-green);
    background: var(--accent-green-dim);
    padding: 3px 8px;
    border-radius: 4px;
    font-family: var(--font-mono);
  }

  .time-range {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .tr-val {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    background: var(--bg-card);
    border: 1px solid var(--border);
    padding: 6px 10px;
    border-radius: 4px;
  }

  .tr-sep {
    color: var(--text-muted);
    font-size: 14px;
  }

  .edit-btns {
    padding: 0 18px 18px;
    display: flex;
    gap: 8px;
  }

  .btn-save {
    flex: 1;
    background: var(--accent-blue);
    border: none;
    border-radius: var(--radius-sm);
    padding: 11px;
    font-size: 13px;
    font-weight: 600;
    color: white;
    cursor: pointer;
    font-family: var(--font-body);
    text-align: center;
  }

  .btn-skip {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 11px 14px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    font-family: var(--font-body);
  }
</style>
