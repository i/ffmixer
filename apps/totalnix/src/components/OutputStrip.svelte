<!--
  Hardware output strip (row 3). Pressing anywhere on the strip selects
  this output's submix (manual §25.2); the selected output is bright,
  others darkened. The name button is the keyboard-accessible way to
  select. Selecting only changes what the UI edits, never the sound.
-->
<script lang="ts">
  import { send } from '../lib/backend.svelte'
  import { formatGain } from '../lib/scale'
  import type { OutputView } from '../lib/types'
  import Fader from './Fader.svelte'
  import Knob from './Knob.svelte'
  import LevelReadout from './LevelReadout.svelte'
  import Meter from './Meter.svelte'
  import SettingsButton from './SettingsButton.svelte'

  interface Props {
    output: OutputView
    selected: boolean
  }

  let { output, selected }: Props = $props()
  let settingsOpen = $state(false)

  function select(): void {
    if (!selected) send({ type: 'selectOutput', output: output.pair })
  }

  function setPhase(side: number): void {
    send({ type: 'setOutputPhase', channel: 2 * output.pair + side, value: !output.phase[side] })
  }
</script>

<div class="strip-group">
  <!--
    Mouse convenience only: the name button below offers the same action
    to keyboard and screen-reader users.
  -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="strip"
    class:selected
    class:silenced={output.muted}
    onpointerdowncapture={select}
  >
    <button
      class="name"
      aria-pressed={selected}
      title={`Select submix ${output.name}`}
      onclick={select}
    >{output.name}</button>
    <Knob
      value={output.balance}
      label={`Balance ${output.name}`}
      onchange={(pan) => send({ type: 'setOutputBalance', output: output.pair, pan })}
    />
    <div class="buttons">
      <button
        class="toggle mute"
        aria-pressed={output.muted}
        title="Mute output"
        onclick={() => send({ type: 'setOutputMuted', output: output.pair, value: !output.muted })}
      >M</button>
      <button class="toggle cue" disabled title="Cue arrives with the Control Room section">CUE</button>
    </div>
    <LevelReadout row="outputs" indices={output.meters} />
    <div class="body">
      <Meter row="outputs" indices={output.meters} />
      <Fader
        db={output.volumeDb}
        label={`${output.name} volume`}
        onchange={(db) => send({ type: 'setOutputVolume', output: output.pair, db })}
      />
      <SettingsButton open={settingsOpen} ontoggle={() => (settingsOpen = !settingsOpen)} />
    </div>
    <div class="gain" class:off={output.volumeDb === null}>{formatGain(output.volumeDb)}</div>
  </div>

  {#if settingsOpen}
    <div class="settings" aria-label={`${output.name} settings`}>
      <div class="settings-title">Settings</div>
      <button
        class="toggle wide"
        aria-pressed="true"
        disabled
        title="Mono hardware outputs are not supported yet"
      >Stereo</button>
      <div class="phase-pair">
        <button
          class="toggle phase"
          aria-pressed={output.phase[0]}
          title="Phase L: invert everything sent to the left output"
          onclick={() => setPhase(0)}
        >Ø L</button>
        <button
          class="toggle phase"
          aria-pressed={output.phase[1]}
          title="Phase R: invert everything sent to the right output"
          onclick={() => setPhase(1)}
        >R Ø</button>
      </div>
      <button
        class="toggle wide"
        disabled
        title="Talkback arrives with the Control Room section"
      >+Talkback</button>
      <button
        class="toggle wide"
        disabled
        title="No Trim arrives with the Trim Gains function"
      >No Trim</button>
      <button
        class="toggle wide"
        disabled
        title="Loopback needs hardware writes (FF800 record mask register, untested)"
      >Loopback</button>
    </div>
  {/if}
</div>

<style>
  .strip {
    display: flex;
    flex-direction: column;
    gap: 3px;
    width: 66px;
    padding: 4px 3px 3px;
    background: var(--strip-dark);
    cursor: pointer;
    border-right: 1px solid var(--strip-edge);
    box-sizing: border-box;
    flex-shrink: 0;
  }
  .strip.selected {
    background: var(--strip-selected);
  }
  .strip.silenced .body {
    opacity: 0.45;
  }
  .name {
    font-size: 11px;
    padding: 2px 0;
    border: none;
    border-radius: 3px;
    color: var(--text);
    background: transparent;
    cursor: pointer;
    white-space: nowrap;
  }
  .name:hover {
    background: rgba(255, 255, 255, 0.06);
  }
  .name[aria-pressed='true'] {
    color: #111;
    background: var(--accent);
    font-weight: 600;
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
  }
  .gain {
    font-size: 10px;
    text-align: center;
    border-radius: 3px;
    background: #1a1d21;
    padding: 1px 0;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }
  .off {
    color: var(--text-dim);
  }
</style>
