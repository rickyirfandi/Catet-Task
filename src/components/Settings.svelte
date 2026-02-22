<script lang="ts">
  import { getUser, logout, getInitials } from '$lib/stores/auth.svelte';
  import { getSetting, setSetting } from '$lib/api/tauri';
  import { onMount } from 'svelte';

  let roundDuration = $state(15);
  let launchAtLogin = $state(false);

  onMount(async () => {
    try {
      const rd = await getSetting('round_duration');
      if (rd) roundDuration = parseInt(rd);
      const lal = await getSetting('launch_at_login');
      if (lal) launchAtLogin = lal === 'true';
    } catch {
      // Settings not yet set
    }
  });

  async function setRoundDuration(val: number) {
    roundDuration = val;
    await setSetting('round_duration', String(val));
  }

  async function toggleLaunchAtLogin() {
    launchAtLogin = !launchAtLogin;
    await setSetting('launch_at_login', String(launchAtLogin));
  }

  async function handleLogout() {
    await logout();
  }

  let user = $derived(getUser());
</script>

<div class="settings">
  {#if user}
    <div class="profile-section">
      <div class="profile-card">
        <div class="avatar">{getInitials()}</div>
        <div class="profile-info">
          <div class="profile-name">{user.displayName}</div>
          <div class="profile-email">{user.email}</div>
          <div class="profile-domain">{user.jiraDomain} &middot; API Token</div>
        </div>
      </div>
    </div>
  {/if}

  <div class="settings-list">
    <div class="setting-item">
      <div class="setting-info">
        <div class="setting-name">Round Durations</div>
        <div class="setting-desc">Auto-round worklogs before submitting</div>
      </div>
      <button class="setting-val" onclick={() => setRoundDuration(roundDuration === 15 ? 30 : roundDuration === 30 ? 1 : 15)}>
        {roundDuration} min
      </button>
    </div>

    <div class="setting-item disabled">
      <div class="setting-info">
        <div class="setting-name">Idle Detection</div>
        <div class="setting-desc">Detect AFK and prompt to keep/discard</div>
      </div>
      <div class="toggle"></div>
    </div>

    <div class="setting-item disabled">
      <div class="setting-info">
        <div class="setting-name">Idle Threshold</div>
        <div class="setting-desc">Minutes before showing idle prompt</div>
      </div>
      <span class="setting-val-static">5 min</span>
    </div>

    <div class="setting-item disabled">
      <div class="setting-info">
        <div class="setting-name">Daily Reminder</div>
        <div class="setting-desc">Notify if unlogged entries at end of day</div>
      </div>
      <div class="toggle"></div>
    </div>

    <div class="setting-item disabled">
      <div class="setting-info">
        <div class="setting-name">Reminder Time</div>
        <div class="setting-desc">When to send the reminder notification</div>
      </div>
      <span class="setting-val-static">17:00</span>
    </div>

    <div class="setting-item">
      <div class="setting-info">
        <div class="setting-name">Launch at Login</div>
        <div class="setting-desc">Start JTT when you log in to your computer</div>
      </div>
      <button class="toggle" class:on={launchAtLogin} onclick={toggleLaunchAtLogin}></button>
    </div>
  </div>

  <button class="btn-danger" onclick={handleLogout}>Disconnect &amp; Logout</button>
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
  }

  .profile-section {
    padding: 14px 14px 8px;
  }

  .profile-card {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: linear-gradient(135deg, #6366f1, #8b5cf6);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    font-weight: 700;
    color: white;
    flex-shrink: 0;
  }

  .profile-info {
    flex: 1;
  }

  .profile-name {
    font-size: 14px;
    font-weight: 600;
  }

  .profile-email {
    font-size: 11px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
  }

  .profile-domain {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .settings-list {
    padding: 14px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .setting-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .setting-item.disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .setting-info {
    flex: 1;
  }

  .setting-name {
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 2px;
  }

  .setting-desc {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .setting-val {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--accent-blue);
    font-weight: 600;
    flex-shrink: 0;
    margin-left: 12px;
    background: rgba(61, 122, 237, 0.08);
    padding: 4px 10px;
    border-radius: 4px;
    border: none;
    cursor: pointer;
  }

  .setting-val-static {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--accent-blue);
    font-weight: 600;
    flex-shrink: 0;
    margin-left: 12px;
    background: rgba(61, 122, 237, 0.08);
    padding: 4px 10px;
    border-radius: 4px;
  }

  .toggle {
    width: 40px;
    height: 22px;
    border-radius: 11px;
    background: var(--border);
    position: relative;
    cursor: pointer;
    flex-shrink: 0;
    margin-left: 12px;
    transition: background 0.2s;
    border: none;
  }

  .toggle.on {
    background: var(--accent-green);
  }

  .toggle::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    transition: transform 0.2s;
  }

  .toggle.on::after {
    transform: translateX(18px);
  }

  .btn-danger {
    margin: 0 14px 14px;
    background: transparent;
    border: 1px solid var(--accent-red);
    border-radius: var(--radius-sm);
    padding: 10px;
    font-size: 13px;
    font-weight: 500;
    color: var(--accent-red);
    cursor: pointer;
    font-family: var(--font-body);
    width: calc(100% - 28px);
    text-align: center;
  }

  .btn-danger:hover {
    background: var(--accent-red-dim);
  }
</style>
