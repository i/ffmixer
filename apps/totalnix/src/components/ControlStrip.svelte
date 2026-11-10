<!-- Fixed right-hand Control Strip (manual §25.5). -->
<script lang="ts">
  import { send, session } from '../lib/backend.svelte'
  import { viewOptions } from '../lib/viewOptions.svelte'
  import type { ViewState } from '../lib/types'

  interface Props {
    view: ViewState
  }

  let { view }: Props = $props()

  const modeLabel = $derived(
    { loading: 'Loading…', hardware: 'Read-only', demo: 'Demo' }[view.mode],
  )
</script>

<aside class="control-strip" aria-label="Control strip">
  <div class="device">
    <div class="device-name">Fireface 800</div>
    <div class="device-id">{view.device}</div>
    <div class="badge" data-mode={view.mode}>{modeLabel}</div>
  </div>

  <p class="status">{view.status}</p>
  {#if view.pendingChanges !== null}
    <p class="pending" title="Crosspoints that would change on the unit if writes were enabled">
      {view.pendingChanges === 0 ? 'In sync with unit' : `${view.pendingChanges} unsent changes`}
    </p>
  {/if}
  {#if session.error}
    <p class="error" role="alert">{session.error}</p>
  {/if}

  <div class="masters" role="group" aria-label="Global mute, solo and fader group">
    <button
      class="toggle mute"
      aria-pressed={view.anyMuted}
      title="Mute master: switch all mutes off, click again to restore"
      onclick={() => send({ type: 'toggleMuteMaster' })}
    >M</button>
    <button
      class="toggle solo"
      aria-pressed={view.anySoloed}
      title="Solo master: switch all solos off, click again to restore"
      onclick={() => send({ type: 'toggleSoloMaster' })}
    >S</button>
    <button class="toggle" disabled title="Temporary fader group (coming later)">F</button>
  </div>

  <section class="panel" aria-label="View options">
    <h2>View Options</h2>
    <h3>Routing Mode</h3>
    <div class="pair">
      <button class="toggle wide" aria-pressed="true">Submix</button>
      <button class="toggle wide" disabled title="Free routing mode (coming later)">Free</button>
    </div>
    <h3>Level Meters</h3>
    <div class="pair">
      <button class="toggle wide" aria-pressed={viewOptions.rms} onclick={() => (viewOptions.rms = !viewOptions.rms)}>RMS</button>
    </div>
    <h3>Show</h3>
    <div class="pair">
      <button class="toggle wide" aria-pressed={viewOptions.twoRow} onclick={() => (viewOptions.twoRow = !viewOptions.twoRow)}>2 Row</button>
    </div>
  </section>

  {#if view.warnings.length > 0}
    <section class="panel warnings" aria-label="Import warnings">
      <h2>Import notes</h2>
      <ul>
        {#each view.warnings as warning (warning)}
          <li>{warning}</li>
        {/each}
      </ul>
    </section>
  {/if}
</aside>

<style>
  .control-strip {
    width: 168px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    background: var(--panel-dark);
    border-left: 1px solid #111;
    overflow-y: auto;
    box-sizing: border-box;
  }
  .device {
    background: var(--panel);
    border-radius: 4px;
    padding: 6px 8px;
  }
  .device-name {
    font-size: 12px;
    color: var(--text);
    font-weight: 600;
  }
  .device-id {
    font-size: 10px;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .badge {
    display: inline-block;
    margin-top: 4px;
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 8px;
    color: #111;
    background: #8fa0b0;
  }
  .badge[data-mode='hardware'] {
    background: #79c46a;
  }
  .badge[data-mode='demo'] {
    background: var(--accent);
  }
  .status,
  .pending,
  .error {
    margin: 0;
    font-size: 10.5px;
    line-height: 1.35;
    color: var(--text-dim);
  }
  .pending {
    color: var(--text);
  }
  .error {
    color: var(--over);
  }
  .masters {
    display: flex;
    gap: 3px;
  }
  .masters .toggle {
    flex: 1;
    height: 22px;
  }
  .panel {
    background: var(--panel-blue);
    border-radius: 4px;
    padding: 6px 8px 8px;
  }
  .panel h2 {
    margin: 0 0 4px;
    font-size: 11px;
    color: #e8f6ff;
  }
  .panel h3 {
    margin: 6px 0 3px;
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #cfeeff;
  }
  .pair {
    display: flex;
    gap: 3px;
  }
  .pair .toggle {
    flex: 1;
  }
  .warnings {
    background: var(--panel);
  }
  .warnings ul {
    margin: 0;
    padding-left: 14px;
    font-size: 10px;
    color: var(--text-dim);
  }
</style>
