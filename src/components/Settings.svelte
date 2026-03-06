<script lang="ts">
  import { getUser, logout, getInitials } from '$lib/stores/auth.svelte';
  import {
    getSetting, setSetting, setLaunchAtLogin, quitApp, resetTimerData,
    getCliStatus, installCli, uninstallCli,
    getClaudeDesktopStatus, connectClaudeDesktop, disconnectClaudeDesktop,
    type CliStatus, type ClaudeDesktopStatus,
  } from '$lib/api/tauri';
  import { onMount } from 'svelte';

  let roundDuration = $state(15);
  let launchAtLogin = $state(false);
  let dailyReminder = $state(false);
  let reminderTime = $state('17:00');
  let localTimezone = $state('Local time');

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
      refreshIntegrationStatus();
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

  // ── CLI + Integrations state ──
  let cliStatus = $state<CliStatus | null>(null);
  let claudeStatus = $state<ClaudeDesktopStatus | null>(null);
  let cliLoading = $state(false);
  let claudeLoading = $state(false);
  let cliError = $state('');
  let claudeError = $state('');

  async function refreshIntegrationStatus() {
    try { cliStatus = await getCliStatus(); } catch { /* ignore */ }
    try { claudeStatus = await getClaudeDesktopStatus(); } catch { /* ignore */ }
  }

  async function handleInstallCli() {
    cliLoading = true;
    cliError = '';
    try {
      await installCli();
      cliStatus = await getCliStatus();
    } catch (e) {
      cliError = String(e);
    } finally {
      cliLoading = false;
    }
  }

  async function handleUninstallCli() {
    cliLoading = true;
    cliError = '';
    try {
      await uninstallCli();
      cliStatus = await getCliStatus();
    } catch (e) {
      cliError = String(e);
    } finally {
      cliLoading = false;
    }
  }

  async function handleConnectClaude() {
    claudeLoading = true;
    claudeError = '';
    try {
      await connectClaudeDesktop();
      claudeStatus = await getClaudeDesktopStatus();
    } catch (e) {
      claudeError = String(e);
    } finally {
      claudeLoading = false;
    }
  }

  async function handleDisconnectClaude() {
    claudeLoading = true;
    claudeError = '';
    try {
      await disconnectClaudeDesktop();
      claudeStatus = await getClaudeDesktopStatus();
    } catch (e) {
      claudeError = String(e);
    } finally {
      claudeLoading = false;
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
        <div class="setting-time-row">
          <input
            class="setting-time"
            type="time"
            value={reminderTime}
            step="60"
            disabled={!dailyReminder}
            aria-label="Daily reminder time"
            onchange={(e) => setReminderTime((e.target as HTMLInputElement).value)}
          />
        </div>
        <div class="setting-time-note">Selected: {formatReminderTime(reminderTime)} ({localTimezone})</div>
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

  <!-- ── Integrations ── -->
  <div class="integrations-section">
    <div class="integrations-title">Integrations</div>

    <!-- CLI Tools -->
    <div class="integration-card">
      <div class="integration-header">
        <div class="integration-info">
          <div class="integration-name">CLI Tools</div>
          <div class="integration-desc">Use <code>catet-cli</code> in Terminal for scripting &amp; automation</div>
        </div>
        {#if cliStatus?.installed}
          <span class="badge-connected">Installed</span>
        {:else}
          <span class="badge-disconnected">Not installed</span>
        {/if}
      </div>

      {#if cliStatus?.installed}
        <div class="integration-path">{cliStatus.installPath}</div>
        <button class="btn-integration-secondary" onclick={handleUninstallCli} disabled={cliLoading}>
          {cliLoading ? 'Removing…' : 'Uninstall CLI'}
        </button>
      {:else}
        {#if !cliStatus?.cliBinaryFound}
          <div class="integration-hint">Build first: <code>cd catet-cli &amp;&amp; cargo build --release</code></div>
        {/if}
        <button
          class="btn-integration-primary"
          onclick={handleInstallCli}
          disabled={cliLoading || !cliStatus?.cliBinaryFound}
        >
          {cliLoading ? 'Installing…' : 'Install CLI Tools'}
        </button>
      {/if}
      {#if cliError}
        <div class="integration-error">{cliError}</div>
      {/if}
    </div>

    <!-- Claude Desktop -->
    <div class="integration-card">
      <div class="integration-header">
        <div class="integration-info">
          <div class="integration-name">Claude Desktop</div>
          <div class="integration-desc">Let Claude read your time data and log work to Jira</div>
        </div>
        {#if claudeStatus?.connected}
          <span class="badge-connected">Connected</span>
        {:else if claudeStatus?.claudeInstalled}
          <span class="badge-disconnected">Not connected</span>
        {:else}
          <span class="badge-unavailable">Not found</span>
        {/if}
      </div>

      {#if claudeStatus?.connected}
        <div class="integration-hint">Restart Claude Desktop to apply any changes.</div>
        <button class="btn-integration-secondary" onclick={handleDisconnectClaude} disabled={claudeLoading}>
          {claudeLoading ? 'Disconnecting…' : 'Disconnect'}
        </button>
      {:else if claudeStatus?.claudeInstalled}
        <button
          class="btn-integration-primary"
          onclick={handleConnectClaude}
          disabled={claudeLoading || !cliStatus?.cliBinaryFound}
        >
          {claudeLoading ? 'Connecting…' : 'Connect to Claude Desktop'}
        </button>
        {#if !cliStatus?.cliBinaryFound}
          <div class="integration-hint">Install CLI Tools first.</div>
        {:else}
          <div class="integration-hint">Restart Claude Desktop after connecting.</div>
        {/if}
      {:else}
        <div class="integration-hint">Claude Desktop not found. <a class="integration-link" href="https://claude.ai/download" target="_blank" rel="noopener">Download it here.</a></div>
      {/if}
      {#if claudeError}
        <div class="integration-error">{claudeError}</div>
      {/if}
    </div>
  </div>

  <button class="btn-reset" onclick={() => showResetConfirm = true}>Reset Data</button>
  <button class="btn-danger" onclick={handleLogout}>Disconnect &amp; Logout</button>
  <button class="btn-quit" onclick={handleQuit}>Quit Catet Task</button>

  <p class="byline">2026 - Ricky Irfandi</p>

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
    color: var(--text-primary);
    font-weight: 600;
    flex-shrink: 0;
    margin-left: 0;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    padding: 5px 8px;
    border-radius: 7px;
    width: 108px;
  }

  .setting-time-group {
    margin-left: 12px;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
    flex-shrink: 0;
    min-width: 108px;
  }

  .setting-time-row {
    width: 100%;
    display: flex;
    justify-content: flex-end;
  }

  .setting-time:disabled {
    opacity: 0.65;
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
    opacity: 1;
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

  /* ── Integrations ── */

  .integrations-section {
    padding: 0 14px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .integrations-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 0 2px 4px;
  }

  .integration-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .integration-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
  }

  .integration-info {
    flex: 1;
  }

  .integration-name {
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 2px;
  }

  .integration-desc {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .integration-desc code {
    font-family: var(--font-mono);
    color: var(--text-secondary);
    font-size: 10px;
    background: rgba(255,255,255,0.05);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .badge-connected {
    font-size: 10px;
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--accent-green);
    background: rgba(45, 212, 160, 0.1);
    border: 1px solid rgba(45, 212, 160, 0.25);
    padding: 2px 8px;
    border-radius: 20px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .badge-disconnected {
    font-size: 10px;
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--text-muted);
    background: rgba(255,255,255,0.04);
    border: 1px solid var(--border);
    padding: 2px 8px;
    border-radius: 20px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .badge-unavailable {
    font-size: 10px;
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--text-muted);
    background: rgba(255,255,255,0.04);
    border: 1px solid var(--border);
    padding: 2px 8px;
    border-radius: 20px;
    white-space: nowrap;
    flex-shrink: 0;
    opacity: 0.5;
  }

  .btn-integration-primary {
    width: 100%;
    padding: 9px;
    background: var(--accent-blue);
    border: none;
    border-radius: var(--radius-sm);
    color: white;
    font-size: 12px;
    font-weight: 600;
    font-family: var(--font-body);
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .btn-integration-primary:hover:not(:disabled) {
    opacity: 0.85;
  }

  .btn-integration-primary:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .btn-integration-secondary {
    width: 100%;
    padding: 8px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    font-family: var(--font-body);
    cursor: pointer;
  }

  .btn-integration-secondary:hover:not(:disabled) {
    border-color: var(--text-muted);
    color: var(--text-primary);
  }

  .btn-integration-secondary:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .integration-path {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--accent-green);
    word-break: break-all;
  }

  .integration-hint {
    font-size: 10px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .integration-hint code {
    font-family: var(--font-mono);
    color: var(--text-secondary);
    font-size: 10px;
    background: rgba(255,255,255,0.05);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .integration-link {
    color: var(--accent-blue);
    text-decoration: none;
  }

  .integration-link:hover {
    text-decoration: underline;
  }

  .integration-error {
    font-size: 11px;
    color: var(--accent-red);
    line-height: 1.4;
    word-break: break-word;
  }
</style>
