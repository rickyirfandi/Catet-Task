<script lang="ts">
  interface Props {
    checked: boolean;
    partial?: boolean;
    onchange?: () => void;
  }

  let { checked, partial = false, onchange }: Props = $props();
</script>

<button
  class="checkbox"
  class:checked
  class:partial={partial && !checked}
  onclick={onchange}
  type="button"
  aria-checked={checked}
  role="checkbox"
>
  {#if checked}
    &#10003;
  {:else if partial}
    &mdash;
  {/if}
</button>

<style>
  .checkbox {
    width: 20px;
    height: 20px;
    border-radius: 5px;
    border: 2px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    color: transparent;
    flex-shrink: 0;
    cursor: pointer;
    background: none;
    transition: all 0.15s;
  }

  .checkbox.checked {
    background: var(--accent-blue);
    border-color: var(--accent-blue);
    color: white;
  }

  .checkbox.partial {
    border-color: var(--accent-blue);
    color: var(--accent-blue);
  }
</style>
