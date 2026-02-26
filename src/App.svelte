<script lang="ts">
  import { isLoggedIn, tryAutoLogin, getLoading as getAuthLoading } from '$lib/stores/auth.svelte';
  import Login from './components/Login.svelte';
  import Panel from './components/Panel.svelte';
  import appIcon from './assets/app-icon.png';
  import { onMount } from 'svelte';

  onMount(() => {
    tryAutoLogin();
  });
</script>

{#if getAuthLoading()}
  <div class="splash">
    <img class="splash-logo" src={appIcon} alt="Catet Task" />
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
    object-fit: contain;
    animation: pulse 2s ease-in-out infinite;
  }
</style>
