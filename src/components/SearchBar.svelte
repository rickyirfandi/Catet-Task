<script lang="ts">
  import { getSearchQuery, setSearchQuery, search, getProjectKeys, getActiveProjectFilter, setActiveProjectFilter } from '$lib/stores/tasks.svelte';
  import { getProjectColor } from '$lib/utils/projectColor';
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

  let projectKeys = $derived(getProjectKeys());
  let activeFilter = $derived(getActiveProjectFilter());

  function handleChipClick(key: string | null) {
    setActiveProjectFilter(activeFilter === key ? null : key);
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

{#if projectKeys.length > 1}
  <div class="chip-row">
    <button
      class="chip"
      class:active={activeFilter === null}
      onclick={() => handleChipClick(null)}
    >All</button>
    {#each projectKeys as key (key)}
      <button
        class="chip"
        class:active={activeFilter === key}
        onclick={() => handleChipClick(key)}
      >
        <span class="chip-dot" style:background={getProjectColor(key)}></span>
        {key}
      </button>
    {/each}
  </div>
{/if}

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

  .chip-row {
    display: flex;
    gap: 6px;
    padding: 0 14px 4px;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .chip-row::-webkit-scrollbar {
    display: none;
  }

  .chip {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 3px 10px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-muted);
    font-size: 10px;
    font-weight: 600;
    font-family: var(--font-mono);
    letter-spacing: 0.3px;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s;
  }

  .chip:hover {
    border-color: var(--text-secondary);
    color: var(--text-secondary);
  }

  .chip.active {
    background: var(--accent-blue);
    border-color: var(--accent-blue);
    color: #0d0f13;
  }

  .chip-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
</style>
