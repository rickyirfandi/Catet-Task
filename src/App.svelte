<script lang="ts">
  import { isLoggedIn, tryAutoLogin, getLoading as getAuthLoading } from '$lib/stores/auth.svelte';
  import Login from './components/Login.svelte';
  import Panel from './components/Panel.svelte';
  import { onMount } from 'svelte';

  onMount(() => {
    tryAutoLogin();
  });
</script>

{#if getAuthLoading()}
  <div class="splash">
    <div class="splash-logo">JT</div>
  </div>
{:else if isLoggedIn()}
  <Panel />
{:else}
  <Login />
{/if}

<style>
  .splash {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
  }
  .splash-logo {
    width: 48px;
    height: 48px;
    border-radius: 14px;
    background: linear-gradient(135deg, var(--accent-blue), #6366f1);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 22px;
    font-weight: 700;
    color: white;
    font-family: var(--font-mono);
    animation: pulse 2s ease-in-out infinite;
  }
</style>
