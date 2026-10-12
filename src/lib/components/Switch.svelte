<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    id: string;
    label: string;
    hint?: string;
    checked: boolean;
    disabled?: boolean;
    onchange: (checked: boolean) => void;
    /** Extra content under the hint (status line, inputs shown when enabled, …). */
    children?: Snippet;
  }

  let { id, label, hint, checked, disabled = false, onchange, children }: Props = $props();
</script>

<div class="switch-row">
  <div class="text">
    <label for={id}>{label}</label>
    {#if hint}<p class="hint" id="{id}-hint">{hint}</p>{/if}
  </div>
  <input
    {id}
    type="checkbox"
    role="switch"
    class="switch"
    {checked}
    {disabled}
    aria-describedby={hint ? `${id}-hint` : undefined}
    onchange={(e) => {
      const input = e.currentTarget;
      onchange(input.checked);
      // Controlled input: show the state the owner accepted (an async or failed change snaps back).
      input.checked = checked;
    }}
  />
</div>
{#if children}
  <div class="extra">{@render children()}</div>
{/if}

<style>
  .switch-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  label {
    font-weight: 600;
    cursor: pointer;
  }

  .switch {
    flex: none;
    appearance: none;
    position: relative;
    width: 46px;
    height: 26px;
    margin: 2px 0 0;
    border-radius: 13px;
    background: rgba(255, 236, 214, 0.14);
    border: 1px solid var(--border-strong);
    cursor: pointer;
    transition: background-color 0.18s ease;
  }

  .switch::before {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--text-2);
    transition:
      transform 0.18s ease,
      background-color 0.18s ease;
  }

  .switch:checked {
    background: var(--accent);
    border-color: var(--accent);
  }

  .switch:checked::before {
    transform: translateX(20px);
    background: var(--accent-ink);
  }

  .switch:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .switch:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .extra {
    margin-top: 12px;
  }

  .extra:empty {
    display: none;
  }
</style>
