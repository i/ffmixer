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
  import SettingsButton from './SettingsButton.svelte'

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

  function setPhase(side: number): void {
    send({
      type: 'setPhase',
      kind: strip.kind,
      channel: strip.channel + side,
      value: !strip.phase[side],
    })
  }
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
      <SettingsButton open={settingsOpen} ontoggle={() => (settingsOpen = !settingsOpen)} />
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
        title="Stereo or two mono channels"
        onclick={() => send({ type: 'setStereo', kind: strip.kind, pair, value: !strip.stereo })}
      >Stereo</button>
      {#if strip.stereo}
        <Knob
          value={strip.width}
          label={`Width ${strip.name}`}
          format={(width) => width.toFixed(2)}
          resetValue={1}
          onchange={(width) => send({ type: 'setWidth', kind: strip.kind, pair, width })}
        />
        <div class="settings-caption">Width</div>
        <button
          class="toggle wide"
          aria-pressed={strip.msProc}
          title="M/S processing: mid to the left channel, side to the right (all routings)"
          onclick={() => send({ type: 'setMsProc', kind: strip.kind, pair, value: !strip.msProc })}
        >MS Proc</button>
        <div class="phase-pair">
          <button
            class="toggle phase"
            aria-pressed={strip.phase[0]}
            title="Phase L: invert the left channel (all routings)"
            onclick={() => setPhase(0)}
          >Ø L</button>
          <button
            class="toggle phase"
            aria-pressed={strip.phase[1]}
            title="Phase R: invert the right channel (all routings)"
            onclick={() => setPhase(1)}
          >R Ø</button>
        </div>
      {:else}
        <button
          class="toggle wide phase"
          aria-pressed={strip.phase[0]}
          title="Phase: invert this channel (all routings)"
          onclick={() => setPhase(0)}
        >Ø Phase</button>
      {/if}
    </div>
  {/if}
</div>

<style>
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
</style>
