<script lang="ts">
  interface Props {
    totalSecs: number;
    onchange: (secs: number) => void;
    showAdjust?: boolean;
    showQuick?: boolean;
  }

  let { totalSecs, onchange, showAdjust = true, showQuick = true }: Props = $props();

  let hours = $derived(Math.floor(Math.max(0, totalSecs) / 3600));
  let minutes = $derived(Math.floor((Math.max(0, totalSecs) % 3600) / 60));
  let hoursInput = $state('0');
  let minutesInput = $state('00');

  $effect(() => {
    hoursInput = String(hours);
    minutesInput = String(minutes).padStart(2, '0');
  });

  function parseWholeNumber(raw: string): number {
    const digits = raw.replace(/[^\d]/g, '');
    if (!digits) return 0;
    const parsed = Number.parseInt(digits, 10);
    return Number.isFinite(parsed) ? parsed : 0;
  }

  function update(h: number, m: number) {
    const totalMinutes = Math.max(0, h * 60 + m);
    onchange(totalMinutes * 60);
  }

  function commitFromInputs() {
    update(parseWholeNumber(hoursInput), parseWholeNumber(minutesInput));
  }

  function adjustMinutes(delta: number) {
    const nextMinutes = Math.max(0, Math.floor(totalSecs / 60) + delta);
    onchange(nextMinutes * 60);
  }

  const quickDurations = [
    { label: '15m', secs: 15 * 60 },
    { label: '30m', secs: 30 * 60 },
    { label: '1h', secs: 3600 },
    { label: '2h', secs: 7200 },
    { label: '2h30', secs: 9000 },
    { label: '4h', secs: 14400 },
  ];
</script>

<div class="dur-editor">
  <div class="dur-col">
    <input
      class="dur-seg-input"
      type="text"
      inputmode="numeric"
      pattern="[0-9]*"
      bind:value={hoursInput}
      oninput={(e) => hoursInput = (e.target as HTMLInputElement).value.replace(/[^\d]/g, '')}
      onblur={commitFromInputs}
      onkeydown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
    />
    <span class="dur-unit">hrs</span>
  </div>
  <span class="dur-sep">:</span>
  <div class="dur-col">
    <input
      class="dur-seg-input"
      type="text"
      inputmode="numeric"
      pattern="[0-9]*"
      bind:value={minutesInput}
      oninput={(e) => minutesInput = (e.target as HTMLInputElement).value.replace(/[^\d]/g, '')}
      onblur={commitFromInputs}
      onkeydown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
    />
    <span class="dur-unit">min</span>
  </div>
  {#if showAdjust}
    <div class="dur-adj">
      <button class="adj-btn" onclick={() => adjustMinutes(15)}>&#9650;</button>
      <button class="adj-btn" onclick={() => adjustMinutes(-15)}>&#9660;</button>
    </div>
  {/if}
</div>

{#if showQuick}
  <div class="quick-durs">
    {#each quickDurations as qd}
      <button class="qd" class:active={totalSecs === qd.secs} onclick={() => onchange(qd.secs)}>{qd.label}</button>
    {/each}
  </div>
{/if}

<style>
  .dur-editor {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .dur-col {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .dur-seg-input {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px 0;
    width: 58px;
    text-align: center;
    font-size: 22px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--text-primary);
    -moz-appearance: textfield;
  }

  .dur-seg-input::-webkit-outer-spin-button,
  .dur-seg-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .dur-sep {
    font-size: 20px;
    font-weight: 600;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .dur-unit {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    text-align: center;
    margin-top: 2px;
  }

  .dur-adj {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-left: 8px;
  }

  .adj-btn {
    width: 28px;
    height: 22px;
    border-radius: 4px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .adj-btn:hover {
    border-color: var(--border-focus);
  }

  .quick-durs {
    display: flex;
    gap: 4px;
    margin-top: 6px;
  }

  .qd {
    font-size: 10px;
    font-weight: 500;
    font-family: var(--font-mono);
    padding: 4px 10px;
    border-radius: 4px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
  }

  .qd:hover {
    border-color: var(--border-focus);
  }

  .qd.active {
    background: rgba(61, 122, 237, 0.12);
    border-color: var(--accent-blue);
    color: var(--accent-blue);
  }
</style>
