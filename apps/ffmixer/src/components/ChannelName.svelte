<!--
  A strip's name field (manual §25.3). Double-click or F2 renames it in
  the UI; Enter or leaving the field keeps the edit, Escape cancels, and
  an empty name restores the channel's own name. Display only: nothing is
  sent to the unit. On output strips a single click also selects the
  submix (`selected` / `onselect`).
-->
<script lang="ts">
  import { tick } from 'svelte'

  const MAX_NAME_CHARS = 24

  interface Props {
    name: string
    defaultName: string
    onrename: (name: string | null) => void
    /** Output strips only: whether this output's submix is selected. */
    selected?: boolean
    onselect?: () => void
  }

  let { name, defaultName, onrename, selected, onselect }: Props = $props()

  let editing = $state(false)
  let draft = $state('')
  let field = $state<HTMLInputElement>()

  const custom = $derived(name !== defaultName)
  const title = $derived(
    [custom ? `${name} (${defaultName})` : name, onselect ? 'click selects the submix' : '', 'double-click to rename']
      .filter(Boolean)
      .join(' · '),
  )

  async function startEditing(): Promise<void> {
    draft = name
    editing = true
    await tick()
    field?.focus()
    field?.select()
  }

  function finish(keep: boolean): void {
    if (!editing) return
    editing = false
    const next = draft.trim()
    if (!keep || next === name) return
    onrename(next === '' || next === defaultName ? null : next)
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'F2') {
      event.preventDefault()
      void startEditing()
    }
  }

  function onfieldkeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') finish(true)
    else if (event.key === 'Escape') finish(false)
  }
</script>

{#if editing}
  <input
    class="name editing"
    bind:this={field}
    bind:value={draft}
    maxlength={MAX_NAME_CHARS}
    aria-label={`Rename ${defaultName}`}
    onkeydown={onfieldkeydown}
    onblur={() => finish(true)}
  />
{:else}
  <button
    class="name"
    class:selectable={onselect !== undefined}
    aria-pressed={selected}
    {title}
    onclick={() => onselect?.()}
    ondblclick={startEditing}
    {onkeydown}
  >{name}</button>
{/if}

<style>
  .name {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
    font: inherit;
    font-size: 11px;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
    padding: 1px 0;
    border: none;
    border-radius: 3px;
    background: transparent;
  }
  button.name {
    cursor: default;
  }
  button.name:hover {
    background: rgba(255, 255, 255, 0.06);
  }
  button.name:focus-visible {
    outline: 1px solid var(--focus);
  }
  button.name.selectable {
    cursor: pointer;
  }
  .name[aria-pressed='true'] {
    color: #111;
    background: var(--accent);
    font-weight: 600;
  }
  .name.editing {
    color: var(--text);
    background: #1a1d21;
    outline: 1px solid var(--focus);
  }
</style>
