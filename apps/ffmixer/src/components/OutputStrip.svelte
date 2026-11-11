<!--
  Hardware output strip (row 3). Pressing anywhere on the strip selects
  this output's submix (manual §25.2); the selected output is bright,
  others darkened. The name button is the keyboard-accessible way to
  select, and double-click renames. Selecting only changes what the UI
  edits, never the sound.

  A pair is one stereo strip or two mono strips, like the source rows. A
  mono output has no balance, and a send into it has no pan.

  AN 1/2 (channels 0 and 1) feed the main monitors, so their faders have
  no double-click / Home jump to 0 dB.
-->
<script lang="ts">
  import { rename, send } from '../lib/backend.svelte'
  import { formatGain } from '../lib/scale'
  import type { OutputView } from '../lib/types'
  import ChannelName from './ChannelName.svelte'
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
    if (!selected) send({ type: 'selectOutput', output: output.channel })
  }

  function setPhase(side: number): void {
    send({ type: 'setOutputPhase', channel: output.channel + side, value: !output.phase[side] })
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
    <ChannelName
      name={output.name}
      defaultName={output.defaultName}
      {selected}
      onselect={select}
      onrename={(name) => rename('output', output.channel, name)}
    />
    {#if output.stereo}
      <Knob
        value={output.balance}
        label={`Balance ${output.name}`}
        onchange={(pan) => send({ type: 'setOutputBalance', output: output.channel, pan })}
      />
    {:else}
      <div class="knob-space"></div>
    {/if}
    <div class="buttons">
      <button
        class="toggle mute"
        aria-pressed={output.muted}
        title="Mute output"
        onclick={() => send({ type: 'setOutputMuted', output: output.channel, value: !output.muted })}
      >M</button>
      <button class="toggle cue" disabled title="Cue arrives with the Control Room section">CUE</button>
    </div>
    <LevelReadout row="outputs" indices={output.meters} />
    <div class="body">
      <Meter row="outputs" indices={output.meters} />
      <Fader
        db={output.volumeDb}
        label={`${output.name} volume`}
        onchange={(db) => send({ type: 'setOutputVolume', output: output.channel, db })}
        unityShortcuts={output.pair !== 0}
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
        aria-pressed={output.stereo}
        title="Stereo or two mono outputs"
        onclick={() => send({ type: 'setOutputStereo', pair: output.pair, value: !output.stereo })}
      >Stereo</button>
      {#if output.stereo}
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
      {:else}
        <button
          class="toggle wide phase"
          aria-pressed={output.phase[0]}
          title="Phase: invert everything sent to this output"
          onclick={() => setPhase(0)}
        >Ø Phase</button>
      {/if}
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
        title="Not available yet: the FF800's record mask only silences the input's recording; how the unit sends the output in its place is unknown"
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
  /* Keeps the rows aligned where a mono output has no balance knob. */
  .knob-space {
    height: 30px;
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
