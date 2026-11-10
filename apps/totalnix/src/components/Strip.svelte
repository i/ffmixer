<!--
  Input or playback channel strip (manual §25.3). In Submix view the
  fader, pan and routing field all refer to the selected output.
-->
<script lang="ts">
  import { send } from '../lib/backend.svelte'
  import { formatGain } from '../lib/scale'
  import type { MeterRow, StripView } from '../lib/types'
  import Fader from './Fader.svelte'
  import Knob from './Knob.svelte'
  import LevelReadout from './LevelReadout.svelte'
  import Meter from './Meter.svelte'

  interface Props {
    strip: StripView
    row: MeterRow
    target: string
    soloActive: boolean
  }

  let { strip, row, target, soloActive }: Props = $props()
  let settingsOpen = $state(false)

  const silencedBySolo = $derived(soloActive && !strip.soloed)
  const pair = $derived(Math.floor(strip.channel / 2))
</script>

<div class="strip-group">
  <div
    class="strip"
    class:stereo={strip.stereo}
    class:silenced={strip.muted || silencedBySolo}
    class:unrouted={!strip.routed}
  >
    <div class="name" title={strip.name}>{strip.name}</div>
    <Knob
      value={strip.pan}
      label={`${strip.stereo ? 'Balance' : 'Pan'} ${strip.name}`}
      onchange={(pan) => send({ type: 'setSendPan', kind: strip.kind, channel: strip.channel, pan })}
    />
    <div class="buttons">
      <button
        class="toggle mute"
        aria-pressed={strip.muted}
        title="Mute (all routings)"
        onclick={() => send({ type: 'setMuted', kind: strip.kind, channel: strip.channel, value: !strip.muted })}
      >M</button>
      <button
        class="toggle solo"
        aria-pressed={strip.soloed}
        title="Solo (current submix)"
        onclick={() => send({ type: 'setSoloed', kind: strip.kind, channel: strip.channel, value: !strip.soloed })}
      >S</button>
    </div>
    <LevelReadout {row} indices={strip.meters} />
    <div class="body">
      <Meter {row} indices={strip.meters} />
      <Fader
        db={strip.gainDb}
        label={`${strip.name} to ${target}`}
        onchange={(db) => send({ type: 'setSendGain', kind: strip.kind, channel: strip.channel, db })}
      />
      <button
        class="icon"
        class:open={settingsOpen}
        title="Channel settings"
        aria-expanded={settingsOpen}
        onclick={() => (settingsOpen = !settingsOpen)}
      >
        <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
          <path
            fill="currentColor"
            d="M10.8 1a4.2 4.2 0 0 0-3.9 5.7L1.4 12.2a1.4 1.4 0 0 0 2 2l5.5-5.5A4.2 4.2 0 0 0 14.6 5l-2.4 2.4-2.3-.6-.6-2.3L11.7 2A4.2 4.2 0 0 0 10.8 1z"
          />
        </svg>
      </button>
    </div>
    <div class="gain" class:off={strip.gainDb === null}>{formatGain(strip.gainDb)}</div>
    <div class="route" class:off={strip.gainDb === null} title={`Routing: ${target}`}>{target}</div>
  </div>

  {#if settingsOpen}
    <div class="settings" aria-label={`${strip.name} settings`}>
      <div class="settings-title">Settings</div>
      <button
        class="toggle wide"
        aria-pressed={strip.stereo}
        onclick={() => send({ type: 'setStereo', kind: strip.kind, pair, value: !strip.stereo })}
      >Stereo</button>
    </div>
  {/if}
</div>

<style>
  .strip-group {
    display: flex;
    flex-shrink: 0;
  }
  .strip {
    display: flex;
    flex-direction: column;
    gap: 3px;
    width: 52px;
    padding: 4px 3px 3px;
    background: var(--strip);
    border-right: 1px solid var(--strip-edge);
    box-sizing: border-box;
  }
  .strip.stereo {
    width: 66px;
  }
  .strip.unrouted {
    background: var(--strip-unrouted);
  }
  .strip.silenced .body,
  .strip.silenced .gain {
    opacity: 0.45;
  }
  .name {
    font-size: 11px;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
    padding: 1px 0;
  }
  .buttons {
    display: flex;
    justify-content: center;
    gap: 2px;
  }
  .body {
    flex: 1;
    min-height: 40px;
    display: flex;
    justify-content: center;
    gap: 3px;
    align-items: stretch;
  }
  .body .icon {
    align-self: flex-end;
  }
  .gain,
  .route {
    font-size: 10px;
    text-align: center;
    border-radius: 3px;
    background: #1a1d21;
    padding: 1px 0;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
  }
  .gain {
    color: var(--accent);
  }
  .route {
    color: var(--text);
  }
  .off {
    color: var(--text-dim);
  }
  .settings {
    width: 72px;
    padding: 6px 5px;
    background: var(--panel);
    border-right: 1px solid var(--strip-edge);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .settings-title {
    font-size: 11px;
    color: var(--text);
  }
</style>
