<script lang="ts">
  import { getUser, logout, getInitials } from '$lib/stores/auth.svelte';
  import { getSetting, setSetting, setLaunchAtLogin, quitApp, resetTimerData } from '$lib/api/tauri';
  import { onMount } from 'svelte';

  let roundDuration = $state(15);
  let launchAtLogin = $state(false);
  let dailyReminder = $state(false);
  let reminderTime = $state('17:00');
  let localTimezone = $state('Local time');
  const reminderPresets = ['16:00', '17:00', '18:00'];

  function normalizeReminderTime(value: string): string {
    const match = value.trim().match(/^(\d{1,2}):(\d{1,2})$/);
    if (!match) return '17:00';
    const hh = Number(match[1]);
    const mm = Number(match[2]);
    if (Number.isNaN(hh) || Number.isNaN(mm)) return '17:00';
    if (hh < 0 || hh > 23 || mm < 0 || mm > 59) return '17:00';
    return `${String(hh).padStart(2, '0')}:${String(mm).padStart(2, '0')}`;
  }

  function formatReminderTime(value: string): string {
    const normalized = normalizeReminderTime(value);
    const [hh, mm] = normalized.split(':').map((part) => Number(part));
    const date = new Date();
    date.setHours(hh, mm, 0, 0);
    return new Intl.DateTimeFormat(undefined, { hour: 'numeric', minute: '2-digit' }).format(date);
  }

  onMount(async () => {
    try {
      const rd = await getSetting('round_duration');
      if (rd) roundDuration = parseInt(rd);
      const lal = await getSetting('launch_at_login');
      if (lal) launchAtLogin = lal === 'true';
      const dr = await getSetting('daily_reminder');
      if (dr) dailyReminder = dr === 'true';
      const rt = await getSetting('reminder_time');
      if (rt) reminderTime = normalizeReminderTime(rt);
      localTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone || 'Local time';
    } catch {
      // Settings not yet set
    }
  });

  async function setRoundDuration(val: number) {
    roundDuration = val;
    await setSetting('round_duration', String(val));
  }

  async function toggleDailyReminder() {
    dailyReminder = !dailyReminder;
    await setSetting('daily_reminder', String(dailyReminder));
  }

  async function setReminderTime(value: string) {
    const normalized = normalizeReminderTime(value);
    reminderTime = normalized;
    await setSetting('reminder_time', normalized);
  }

  async function toggleLaunchAtLogin() {
    const newValue = !launchAtLogin;
    launchAtLogin = newValue;
    try {
      await setLaunchAtLogin(newValue);
    } catch {
      launchAtLogin = !newValue;
    }
  }

  let showResetConfirm = $state(false);

  async function handleResetData() {
    try {
      await resetTimerData();
      showResetConfirm = false;
    } catch {
      // Error handled silently — entries may already be empty
      showResetConfirm = false;
    }
  }

  async function handleLogout() {
    await logout();
  }

  async function handleQuit() {
    await quitApp();
  }

  function handleOverlayKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' || event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      showResetConfirm = false;
    }
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

    <div class="setting-item">
      <div class="setting-info">
        <div class="setting-name">Daily Reminder</div>
        <div class="setting-desc">Notify if unlogged entries at end of day</div>
      </div>
      <button
        class="toggle"
        class:on={dailyReminder}
        type="button"
        aria-label="Toggle daily reminder"
        onclick={toggleDailyReminder}
      ></button>
    </div>

    <div class="setting-item" class:disabled={!dailyReminder}>
      <div class="setting-info">
        <div class="setting-name">Reminder Time</div>
        <div class="setting-desc">When to send the reminder notification (local Mac time)</div>
      </div>
      <div class="setting-time-group">
        <input
          class="setting-time"
          type="time"
          value={reminderTime}
          step="60"
          disabled={!dailyReminder}
          aria-label="Daily reminder time"
          onchange={(e) => setReminderTime((e.target as HTMLInputElement).value)}
        />
        <div class="setting-time-presets">
          {#each reminderPresets as preset}
            <button
              class="setting-time-preset"
              class:active={reminderTime === preset}
              onclick={() => setReminderTime(preset)}
              disabled={!dailyReminder}
            >
              {preset}
            </button>
          {/each}
        </div>
        <div class="setting-time-note">{formatReminderTime(reminderTime)} ({localTimezone})</div>
      </div>
    </div>

    <div class="setting-item">
      <div class="setting-info">
        <div class="setting-name">Launch at Login</div>
        <div class="setting-desc">Start Catet Task when you log in to your computer</div>
      </div>
      <button
        class="toggle"
        class:on={launchAtLogin}
        type="button"
        aria-label="Toggle launch at login"
        onclick={toggleLaunchAtLogin}
      ></button>
    </div>
  </div>

  <button class="btn-reset" onclick={() => showResetConfirm = true}>Reset Data</button>
  <button class="btn-danger" onclick={handleLogout}>Disconnect &amp; Logout</button>
  <button class="btn-quit" onclick={handleQuit}>Quit Catet Task</button>

  <p class="byline">made with ❤️ by Ricky Irfandi</p>

  {#if showResetConfirm}
    <div
      class="overlay"
      role="button"
      tabindex="0"
      aria-label="Close reset confirmation"
      onclick={() => showResetConfirm = false}
      onkeydown={handleOverlayKeydown}
    >
      <div
        class="confirm-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="Reset confirmation"
        tabindex="-1"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="confirm-title">Reset All Data?</div>
        <div class="confirm-body">This will stop any running timer and permanently delete all time entries. Task cache and settings will be kept.</div>
        <div class="confirm-actions">
          <button class="confirm-cancel" onclick={() => showResetConfirm = false}>Cancel</button>
          <button class="confirm-reset" onclick={handleResetData}>Reset</button>
        </div>
      </div>
    </div>
  {/if}
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

  .setting-time {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--accent-blue);
    font-weight: 600;
    flex-shrink: 0;
    margin-left: 12px;
    background: rgba(61, 122, 237, 0.08);
    border: 1px solid rgba(61, 122, 237, 0.2);
    padding: 4px 8px;
    border-radius: 4px;
    width: 92px;
  }

  .setting-time-group {
    margin-left: 12px;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    flex-shrink: 0;
  }

  .setting-time:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .setting-time-presets {
    display: flex;
    gap: 4px;
  }

  .setting-time-preset {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px 7px;
    background: transparent;
    cursor: pointer;
  }

  .setting-time-preset.active {
    color: var(--accent-blue);
    border-color: rgba(61, 122, 237, 0.45);
    background: rgba(61, 122, 237, 0.12);
  }

  .setting-time-preset:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .setting-time-note {
    font-size: 10px;
    color: var(--text-muted);
    line-height: 1.3;
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

  .btn-reset {
    margin: 0 14px 10px;
    background: transparent;
    border: 1px solid var(--accent-orange);
    border-radius: var(--radius-sm);
    padding: 10px;
    font-size: 13px;
    font-weight: 500;
    color: var(--accent-orange);
    cursor: pointer;
    font-family: var(--font-body);
    width: calc(100% - 28px);
    text-align: center;
  }

  .btn-reset:hover {
    background: rgba(240, 153, 62, 0.08);
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

  .btn-quit {
    margin: 0 14px 14px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-muted);
    cursor: pointer;
    font-family: var(--font-body);
    width: calc(100% - 28px);
    text-align: center;
  }

  .btn-quit:hover {
    color: var(--text-secondary);
    border-color: var(--text-muted);
  }

  .byline {
    text-align: center;
    font-size: 10px;
    color: var(--text-muted);
    opacity: 0.45;
    margin: 4px 0 16px;
    font-family: var(--font-body);
    letter-spacing: 0.02em;
  }

  .overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .confirm-dialog {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 20px;
    margin: 0 20px;
    max-width: 300px;
  }

  .confirm-title {
    font-size: 15px;
    font-weight: 700;
    margin-bottom: 8px;
  }

  .confirm-body {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin-bottom: 18px;
  }

  .confirm-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .confirm-cancel {
    padding: 8px 16px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-secondary);
    font-size: 13px;
    font-family: var(--font-body);
    cursor: pointer;
  }

  .confirm-cancel:hover {
    border-color: var(--text-muted);
    color: var(--text-primary);
  }

  .confirm-reset {
    padding: 8px 16px;
    border-radius: var(--radius-sm);
    border: none;
    background: var(--accent-red);
    color: white;
    font-size: 13px;
    font-weight: 600;
    font-family: var(--font-body);
    cursor: pointer;
  }

  .confirm-reset:hover {
    opacity: 0.9;
  }
</style>
