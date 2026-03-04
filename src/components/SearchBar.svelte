<script lang="ts">
  import { getSearchQuery, setSearchQuery, search } from '$lib/stores/tasks.svelte';
  import { onDestroy, onMount } from 'svelte';

  let input = $state('');
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let inputEl: HTMLInputElement;

  onMount(() => {
    input = getSearchQuery();
  });

  onDestroy(() => {
    if (debounceTimer) clearTimeout(debounceTimer);
  });

  function handleInput() {
    setSearchQuery(input);
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      if (input.trim()) search(input);
    }, 300);
  }

  export function focus() {
    inputEl?.focus();
  }
</script>

<div class="search-bar">
  <span class="search-icon">&#128269;</span>
  <input
    bind:this={inputEl}
    bind:value={input}
    oninput={handleInput}
    class="search-input"
    type="text"
    placeholder="Search tasks or paste PROJ-123..."
  />
</div>

<style>
  .search-bar {
    margin: 12px 14px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: border-color 0.15s;
  }

  .search-bar:focus-within {
    border-color: var(--border-focus);
  }

  .search-icon {
    font-size: 12px;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    background: none;
    border: none;
    font-size: 12px;
    color: var(--text-primary);
  }
</style>
