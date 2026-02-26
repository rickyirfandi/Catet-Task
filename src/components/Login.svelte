<script lang="ts">
  import { login, getLoading, getError } from '$lib/stores/auth.svelte';
  import appIcon from '../assets/app-icon.png';

  let domain = $state('');
  let email = $state('');
  let token = $state('');

  async function handleSubmit() {
    if (!domain.trim() || !email.trim() || !token.trim()) return;
    try {
      await login(domain.trim(), email.trim(), token.trim());
    } catch {
      // Error is already in the store
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') handleSubmit();
  }
</script>

<div class="login">
  <div class="login-header">
    <img class="login-logo" src={appIcon} alt="Catet Task" />
    <div class="login-title">Connect to Jira</div>
    <div class="login-desc">Track time, log work, stay in flow.</div>
  </div>

  <div class="auth-methods">
    <div class="auth-chip active">API Token</div>
    <div class="auth-chip disabled">OAuth 2.0</div>
    <div class="auth-chip disabled">Server PAT</div>
  </div>

  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form class="login-form" onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} onkeydown={handleKeydown}>
    <div class="input-group">
      <span class="input-label">Jira Domain</span>
      <input
        class="input-field"
        type="text"
        placeholder="company.atlassian.com"
        bind:value={domain}
      />
    </div>
    <div class="input-group">
      <span class="input-label">Email</span>
      <input
        class="input-field"
        type="email"
        placeholder="you@company.com"
        bind:value={email}
      />
    </div>
    <div class="input-group">
      <span class="input-label">API Token</span>
      <input
        class="input-field"
        type="password"
        placeholder="paste your token here"
        bind:value={token}
      />
    </div>

    {#if getError()}
      <div class="error-msg">{getError()}</div>
    {/if}

    <button class="btn-primary" type="submit" disabled={getLoading()}>
      {getLoading() ? 'Connecting...' : 'Connect & Verify →'}
    </button>
  </form>
</div>

<style>
  .login {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .login-header {
    padding: 28px 24px 12px;
    text-align: center;
  }

  .login-logo {
    width: 48px;
    height: 48px;
    border-radius: 14px;
    margin: 0 auto 14px;
    object-fit: contain;
  }

  .login-title {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 4px;
  }

  .login-desc {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .auth-methods {
    padding: 0 24px 16px;
    display: flex;
    gap: 6px;
  }

  .auth-chip {
    font-size: 10px;
    font-weight: 500;
    color: var(--text-muted);
    background: var(--bg-card);
    border: 1px solid var(--border);
    padding: 5px 12px;
    border-radius: 20px;
    font-family: var(--font-mono);
  }

  .auth-chip.active {
    color: var(--accent-blue);
    border-color: var(--accent-blue);
    background: rgba(61, 122, 237, 0.08);
  }

  .auth-chip.disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .login-form {
    padding: 0 24px 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .input-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.5px;
  }

  .input-field {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 9px 12px;
    font-size: 13px;
    color: var(--text-primary);
    font-family: var(--font-mono);
    transition: border-color 0.15s;
  }

  .input-field:focus {
    border-color: var(--border-focus);
  }

  .error-msg {
    font-size: 12px;
    color: var(--accent-red);
    padding: 8px 12px;
    background: var(--accent-red-dim);
    border-radius: var(--radius-sm);
  }

  .btn-primary {
    background: linear-gradient(135deg, var(--accent-blue), #5b8def);
    border: none;
    border-radius: var(--radius-sm);
    padding: 11px;
    font-size: 13px;
    font-weight: 600;
    color: white;
    cursor: pointer;
    font-family: var(--font-body);
    margin-top: 4px;
    transition: opacity 0.15s;
  }

  .btn-primary:hover {
    opacity: 0.9;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
