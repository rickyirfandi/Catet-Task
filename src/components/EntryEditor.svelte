<script lang="ts">
  import DurationInput from './shared/DurationInput.svelte';
  import CommentField from './shared/CommentField.svelte';
  import { getTasks } from '$lib/stores/tasks.svelte';
  import { getEntries } from '$lib/stores/entries.svelte';
  import { updateEntry } from '$lib/api/tauri';
  import { decodeHtmlEntities } from '$lib/utils/text';
  import {
    parseAppDate,
    formatDurationShort,
  } from '$lib/utils/time';
  import type { TimeEntry } from '$lib/types';

  interface Props {
    entry: TimeEntry;
    onback: () => void;
    onsave: () => void;
  }

  let { entry, onback, onsave }: Props = $props();

  let adjustedSecs = $state(0);
  let description = $state('');
  let startedTime = $state('00:00');
  let saveError = $state('');

  let task = $derived(getTasks().find(t => t.id === entry.taskId));
  let rawSecs = $derived(entry.durationSecs ?? 0);
  let canEditStarted = $derived.by(() => {
    const taskEntries = getEntries().filter((e) =>
      e.taskId === entry.taskId && !e.syncedToJira && e.endTime !== null
    );
    if (taskEntries.length === 0) return true;
    const earliest = taskEntries.reduce((a, b) => {
      if (b.startTime < a.startTime) return b;
      if (b.startTime === a.startTime && b.id < a.id) return b;
      return a;
    });
    return earliest.id === entry.id;
  });

  function toLocalTimeInput(value: string): string {
    const parsed = parseAppDate(value);
    if (!parsed) return '00:00';
    const hh = String(parsed.getHours()).padStart(2, '0');
    const mm = String(parsed.getMinutes()).padStart(2, '0');
    return `${hh}:${mm}`;
  }

  function toSqliteUtcWithLocalTime(baseValue: string, hhmm: string): string | null {
    const base = parseAppDate(baseValue);
    if (!base) return null;
    const match = hhmm.trim().match(/^([01]?\d|2[0-3]):([0-5]\d)$/);
    if (!match) return null;
    const h = Number.parseInt(match[1], 10);
    const m = Number.parseInt(match[2], 10);
    const local = new Date(base);
    local.setHours(h, m, 0, 0);
    const iso = local.toISOString(); // UTC
    return iso.slice(0, 19).replace('T', ' ');
  }

  $effect(() => {
    adjustedSecs = entry.adjustedSecs ?? entry.durationSecs ?? 0;
    description = decodeHtmlEntities(entry.description ?? '');
    startedTime = toLocalTimeInput(entry.startTime);
  });

  async function handleSave() {
    saveError = '';
    if (!Number.isFinite(adjustedSecs) || adjustedSecs < 0) {
      saveError = 'Duration must be 0 or more.';
      return;
    }
    const normalizedSecs = Math.floor(adjustedSecs / 60) * 60;
    const startedAt = canEditStarted ? toSqliteUtcWithLocalTime(entry.startTime, startedTime) : null;
    if (canEditStarted && !startedAt) {
      saveError = 'Invalid start time format.';
      return;
    }
    try {
      await updateEntry(entry.id, normalizedSecs, description || null, null, startedAt);
      onsave();
    } catch (e) {
      console.error('Failed to save entry:', e);
      saveError = 'Failed to save entry.';
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
    <div class="duration-row">
      <div class="field duration-field">
        <div class="field-label">
          Duration
          <span class="field-hint">tracked: {formatDurationShort(rawSecs)}</span>
        </div>
        <DurationInput totalSecs={adjustedSecs} onchange={(s) => adjustedSecs = s} showAdjust={false} showQuick={false} />
      </div>

      {#if canEditStarted}
        <div class="field started-field">
          <div class="field-label">Started</div>
          <input
            class="time-val"
            type="time"
            step="60"
            bind:value={startedTime}
          />
        </div>
      {/if}
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
  {#if saveError}
    <div class="save-error">{saveError}</div>
  {/if}
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

  .duration-row {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }

  .duration-field {
    flex: 1;
    min-width: 0;
  }

  .started-field {
    width: 108px;
    flex-shrink: 0;
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

  .time-val {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    background: var(--bg-card);
    border: 1px solid var(--border);
    width: 100%;
    padding: 8px 10px;
    border-radius: 4px;
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

  .save-error {
    padding: 0 18px 12px;
    color: var(--accent-red);
    font-size: 11px;
    font-family: var(--font-mono);
  }
</style>
