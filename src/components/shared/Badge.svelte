<script lang="ts">
  interface Props {
    variant: 'progress' | 'todo' | 'review' | 'running' | 'rounded' | 'logged' | 'error' | 'stopped';
    label?: string;
  }

  let { variant, label }: Props = $props();

  const labels: Record<string, string> = {
    progress: 'In Progress',
    todo: 'To Do',
    review: 'Review',
    running: 'running',
    rounded: 'rounded',
    logged: 'logged',
    error: 'error',
    stopped: 'Stopped',
  };
</script>

<span class="badge badge-{variant}">
  {#if variant === 'running'}
    <span class="dot"></span>
  {/if}
  {label ?? labels[variant] ?? variant}
</span>

<style>
  .badge {
    font-size: 9px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 3px;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.3px;
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .badge-progress {
    background: rgba(61, 122, 237, 0.12);
    color: var(--accent-blue);
  }

  .badge-todo {
    background: rgba(122, 130, 153, 0.12);
    color: var(--text-secondary);
  }

  .badge-review {
    background: var(--accent-purple-dim);
    color: var(--accent-purple);
  }

  .badge-running {
    background: var(--accent-green-dim);
    color: var(--accent-green);
  }

  .badge-rounded {
    background: var(--accent-orange-dim);
    color: var(--accent-orange);
  }

  .badge-logged {
    background: rgba(74, 80, 104, 0.15);
    color: var(--text-muted);
  }

  .badge-error {
    background: var(--accent-red-dim);
    color: var(--accent-red);
  }

  .badge-stopped {
    background: rgba(122, 130, 153, 0.12);
    color: var(--text-secondary);
  }

  .dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--accent-green);
    animation: pulse 2s ease-in-out infinite;
  }
</style>
