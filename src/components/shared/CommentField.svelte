<script lang="ts">
  interface Props {
    value: string;
    onchange: (val: string) => void;
  }

  let { value, onchange }: Props = $props();

  const templates = [
    { emoji: '&#128027;', label: 'Bug fix' },
    { emoji: '&#10024;', label: 'Feature' },
    { emoji: '&#128295;', label: 'Refactor' },
    { emoji: '&#128221;', label: 'Review' },
    { emoji: '&#128270;', label: 'Investigation' },
  ];

  function applyTemplate(emoji: string, label: string) {
    const prefix = `${emoji} ${label}: `;
    if (!value.startsWith(prefix)) {
      onchange(prefix + value);
    }
  }
</script>

<div class="comment-field">
  <textarea
    class="comment-area"
    placeholder="Add a work description..."
    value={value}
    oninput={(e) => onchange((e.target as HTMLTextAreaElement).value)}
  ></textarea>
  <div class="templates">
    {#each templates as tpl}
      <button class="tpl" onclick={() => applyTemplate(tpl.emoji, tpl.label)}>
        {@html tpl.emoji} {tpl.label}
      </button>
    {/each}
  </div>
</div>

<style>
  .comment-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .comment-area {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    min-height: 72px;
    font-size: 13px;
    color: var(--text-primary);
    font-family: var(--font-body);
    line-height: 1.5;
    width: 100%;
    resize: vertical;
    transition: border-color 0.15s;
  }

  .comment-area:focus {
    border-color: var(--border-focus);
  }

  .templates {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .tpl {
    font-size: 10px;
    font-weight: 500;
    padding: 4px 10px;
    border-radius: 20px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.15s;
  }

  .tpl:hover {
    border-color: rgba(167, 139, 250, 0.4);
    color: var(--accent-purple);
  }
</style>
