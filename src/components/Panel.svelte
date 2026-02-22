<script lang="ts">
  import { getInitials } from '$lib/stores/auth.svelte';
  import { init as initTimer } from '$lib/stores/timer.svelte';
  import { refresh as refreshTasks } from '$lib/stores/tasks.svelte';
  import Timer from './Timer.svelte';
  import Today from './Today.svelte';
  import Settings from './Settings.svelte';
  import type { TabId } from '$lib/types';
  import { onMount } from 'svelte';

  let activeTab = $state<TabId>('timer');

  onMount(() => {
    initTimer();
    refreshTasks();
  });
</script>

<div class="panel-container">
  <header class="p-header">
    <div class="tabs">
      <button class="tab" class:active={activeTab === 'timer'} onclick={() => activeTab = 'timer'}>Timer</button>
      <button class="tab" class:active={activeTab === 'today'} onclick={() => activeTab = 'today'}>Today</button>
      <button class="tab" class:active={activeTab === 'settings'} onclick={() => activeTab = 'settings'}>&#9881;</button>
    </div>
    <div class="avatar">{getInitials()}</div>
  </header>

  <div class="panel-body">
    {#if activeTab === 'timer'}
      <Timer />
    {:else if activeTab === 'today'}
      <Today />
    {:else}
      <Settings />
    {/if}
  </div>
</div>

<style>
  .panel-container {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .p-header {
    padding: 16px 18px 12px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
  }

  .tabs {
    display: flex;
    gap: 2px;
    background: var(--bg-card);
    border-radius: var(--radius-sm);
    padding: 2px;
  }

  .tab {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    padding: 5px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-family: var(--font-mono);
    transition: all 0.15s;
    background: none;
    border: none;
  }

  .tab.active {
    background: var(--accent-blue);
    color: white;
  }

  .tab:hover:not(.active) {
    color: var(--text-secondary);
  }

  .avatar {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: linear-gradient(135deg, #6366f1, #8b5cf6);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 700;
    color: white;
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }
</style>
