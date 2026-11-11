<!--
  A labelled drop-down that always shows what the unit holds. Choosing
  an entry is only a request: the field snaps back to the unit's value,
  and changes once the unit reports the new one.
-->
<script lang="ts" generics="T extends string">
  import type { Option } from '../lib/deviceOptions'

  interface Props {
    label: string
    /** What the unit holds; null if it is none of the options. */
    value: T | null
    options: ReadonlyArray<Option<T>>
    disabled: boolean
    onrequest: (value: T) => void
  }

  let { label, value, options, disabled, onrequest }: Props = $props()

  function changed(event: Event & { currentTarget: HTMLSelectElement }): void {
    const select = event.currentTarget
    const requested = select.value as T
    select.value = value ?? ''
    if (requested !== value) onrequest(requested)
  }
</script>

<label class="field">
  <span class="caption">{label}</span>
  <select {disabled} value={value ?? ''} onchange={changed}>
    {#if value === null}
      <option value="" disabled>Other</option>
    {/if}
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
</label>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .caption {
    font-size: 10px;
    color: var(--text-dim);
  }
  select {
    width: 100%;
    height: 20px;
    box-sizing: border-box;
    padding: 0 2px;
    font: inherit;
    font-size: 10.5px;
    color: var(--text);
    background: #1a1d21;
    border: 1px solid #111;
    border-radius: 3px;
  }
  select:focus-visible {
    outline: 1px solid var(--focus);
    outline-offset: 1px;
  }
  select:disabled {
    opacity: 0.5;
  }
</style>
