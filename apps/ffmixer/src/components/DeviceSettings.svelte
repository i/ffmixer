<!--
  The Fireface Settings dialog (manual ch. 7) in the Control Strip:
  analog inputs and levels, phantom power, instrument options, digital
  formats and clock. Values are what FFADO holds for the unit.

  Changes are sent at once, but only while live. A change that can harm
  equipment or hearing (48 V on, level changes, drive, limiter off, clock
  source) first shows its warning and waits for Apply; Cancel has focus.
  The backend asks for the same confirmation, so this is not the only guard.

  Store in Flash and Read Flash are shown but off: FFADO erases and
  rewrites the unit's settings sector from its own copy without checking,
  and "Read" would load flash into FFADO without sending it to the unit.
-->
<script lang="ts">
  import { deviceStatus, session, setDevice, settingWarning } from '../lib/backend.svelte'
  import { preferences, setSkipPhantomWarning } from '../lib/preferences.svelte'
  import {
    ADAT2_OUTPUTS,
    CLOCK_SOURCES,
    INPUT_CHANNELS,
    INPUT_LEVELS,
    INPUT_SOURCES,
    LOCK_LABELS,
    MICS,
    OUTPUT_LEVELS,
    SPDIF_FORMATS,
    SPDIF_INPUTS,
    SYNC_SOURCE_LABELS,
    formatRate,
  } from '../lib/deviceOptions'
  import type { DeviceSettings, Lock, Mic, Setting } from '../lib/types'
  import SelectField from './SelectField.svelte'

  interface Props {
    settings: DeviceSettings | null
    live: boolean
  }

  let { settings, live }: Props = $props()

  interface PendingChange {
    setting: Setting
    description: string
    warning: string
  }

  /** A risky change waiting for Apply or Cancel. */
  let pending = $state<PendingChange | null>(null)
  /** A change is being sent; further changes wait. */
  let busy = $state(false)
  let cancelButton = $state<HTMLButtonElement>()

  const editable = $derived(live && settings !== null && !busy)
  const status = $derived(deviceStatus.value)

  const lockRows = $derived.by((): Array<[string, Lock]> => {
    if (!status) return []
    const rows: Array<[string, Lock]> = [
      ['Word', status.word],
      ['SPDIF', status.spdif],
      ['ADAT 1', status.adat1],
      ['ADAT 2', status.adat2],
    ]
    if (status.tco !== null) rows.push(['TCO', status.tco])
    return rows
  })

  const currentClock = $derived.by((): string => {
    if (!status) return '–'
    if (status.master) return 'Internal'
    return status.autosyncSource ? SYNC_SOURCE_LABELS[status.autosyncSource] : 'No Lock'
  })

  // Cancel, not Apply, gets the focus, so a stray Enter cannot confirm.
  $effect(() => {
    if (pending) cancelButton?.focus()
  })

  // Going offline makes any open confirmation stale.
  $effect(() => {
    if (!live) pending = null
  })

  async function request(
    setting: Setting,
    description: string,
  ): Promise<void> {
    pending = null
    busy = true
    try {
      // Only the 48 V prompt can be switched off; the backend still
      // accepts the change because we say it is confirmed.
      const skipPrompt =
        preferences.skipPhantomWarning && setting.control === 'phantom' && setting.on
      const warning = skipPrompt ? null : await settingWarning(setting)
      if (warning) pending = { setting, description, warning }
      else await setDevice(setting, skipPrompt)
    } catch (error) {
      session.error = String(error)
    } finally {
      busy = false
    }
  }

  async function apply(): Promise<void> {
    if (!pending) return
    const { setting } = pending
    pending = null
    busy = true
    try {
      await setDevice(setting, true)
    } finally {
      busy = false
    }
  }

  function cancel(): void {
    pending = null
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && pending) cancel()
  }

  function togglePhantom(
    mic: Mic,
    label: string,
    state: boolean | null,
  ): void {
    // An unknown state first goes to off, the safe direction.
    const on = state === null ? false : !state
    void request({ control: 'phantom', mic, on }, `${label} 48 V ${on ? 'on' : 'off'}`)
  }
</script>

<svelte:window {onkeydown} />

<section class="device-settings" aria-label="Device settings">
  <h2>Device Settings</h2>

  {#if settings === null}
    <p class="note">FFADO did not report the unit's settings.</p>
  {:else}
    {@const s = settings}
    {#if !live}
      <p class="note">Shown as the unit holds them. Go live to change them.</p>
    {/if}

    {#if pending}
      <div class="confirm" role="alertdialog" aria-label="Confirm change">
        <p class="what">{pending.description}</p>
        <p class="warning">{pending.warning}</p>
        <div class="pair">
          <button class="toggle wide" bind:this={cancelButton} onclick={cancel}>Cancel</button>
          <button class="toggle wide danger" onclick={apply}>Apply</button>
        </div>
      </div>
    {/if}

    <details open>
      <summary>Analog</summary>

      <h3>Inputs</h3>
      {#each INPUT_CHANNELS as { channel, label } (channel)}
        <SelectField
          {label}
          value={s.inputSources[channel]}
          options={INPUT_SOURCES}
          disabled={!editable}
          onrequest={(source) =>
            request(
              { control: 'inputSource', channel, source },
              `${label}: ${INPUT_SOURCES.find((o) => o.value === source)?.label}`,
            )}
        />
      {/each}

      <h3>Level</h3>
      <SelectField
        label="Line In"
        value={s.inputLevel}
        options={INPUT_LEVELS}
        disabled={!editable}
        onrequest={(level) =>
          request(
            { control: 'inputLevel', level },
            `Line In level: ${INPUT_LEVELS.find((o) => o.value === level)?.label}`,
          )}
      />
      <SelectField
        label="Line Out"
        value={s.outputLevel}
        options={OUTPUT_LEVELS}
        disabled={!editable}
        onrequest={(level) =>
          request(
            { control: 'outputLevel', level },
            `Line Out level: ${OUTPUT_LEVELS.find((o) => o.value === level)?.label}`,
          )}
      />

      <h3>Phantom Power</h3>
      <div class="grid">
        {#each MICS as { mic, label }, index (mic)}
          {@const state = s.phantom[index]}
          <button
            class="toggle wide phantom"
            aria-pressed={state === null ? 'mixed' : state}
            disabled={!editable}
            title={state === null
              ? `${label}: state unknown, FFADO cannot report it. Click to switch it off.`
              : `${label} 48 V`}
            onclick={() => togglePhantom(mic, label, state)}
          >{label} 48V</button>
        {/each}
      </div>
      {#if s.phantom.includes(null)}
        <p class="note">
          Dashed: state unknown, FFADO cannot report it. Click switches it off.
        </p>
      {/if}
      <label class="check">
        <input
          type="checkbox"
          checked={preferences.skipPhantomWarning}
          onchange={(event) => setSkipPhantomWarning(event.currentTarget.checked)}
        />
        Don't ask before switching 48 V on
      </label>

      <h3>Instrument (Input 1)</h3>
      <div class="grid">
        <button
          class="toggle wide"
          aria-pressed={s.instrument.drive}
          disabled={!editable}
          title="Drive: adds gain and distortion"
          onclick={() =>
            request(
              { control: 'drive', on: !s.instrument.drive },
              `Drive ${s.instrument.drive ? 'off' : 'on'}`,
            )}
        >Drive</button>
        <button
          class="toggle wide"
          aria-pressed={s.inputSources.ch1 === 'front' ? s.instrument.limiter : true}
          disabled={!editable || s.inputSources.ch1 !== 'front'}
          title={s.inputSources.ch1 === 'front'
            ? 'Limiter: soft limit on the instrument input'
            : 'The limiter is always on unless input 1 uses the front jack'}
          onclick={() =>
            request(
              { control: 'limiter', on: !s.instrument.limiter },
              `Limiter ${s.instrument.limiter ? 'off' : 'on'}`,
            )}
        >Limiter</button>
        <button
          class="toggle wide span"
          aria-pressed={s.instrument.speakerEmulation}
          disabled={!editable}
          title="Speaker Emulation: removes low and high frequencies"
          onclick={() =>
            request(
              { control: 'speakerEmulation', on: !s.instrument.speakerEmulation },
              `Speaker Emulation ${s.instrument.speakerEmulation ? 'off' : 'on'}`,
            )}
        >Speaker Emulation</button>
      </div>
    </details>

    <details>
      <summary>Digital</summary>

      <h3>Options</h3>
      <SelectField
        label="SPDIF Input"
        value={s.spdifInput}
        options={SPDIF_INPUTS}
        disabled={!editable}
        onrequest={(input) =>
          request(
            { control: 'spdifInput', input },
            `SPDIF Input: ${SPDIF_INPUTS.find((o) => o.value === input)?.label}`,
          )}
      />

      <h3>Output Format</h3>
      <SelectField
        label="ADAT 2"
        value={s.adat2Output}
        options={ADAT2_OUTPUTS}
        disabled={!editable}
        onrequest={(output) =>
          request(
            { control: 'adat2Output', output },
            `ADAT 2 output: ${ADAT2_OUTPUTS.find((o) => o.value === output)?.label}`,
          )}
      />
      <SelectField
        label="SPDIF"
        value={s.spdifFormat}
        options={SPDIF_FORMATS}
        disabled={!editable}
        onrequest={(format) =>
          request(
            { control: 'spdifFormat', format },
            `SPDIF format: ${SPDIF_FORMATS.find((o) => o.value === format)?.label}`,
          )}
      />
      <div class="grid">
        <button
          class="toggle wide span"
          aria-pressed={s.emphasis}
          disabled={!editable}
          onclick={() =>
            request({ control: 'emphasis', on: !s.emphasis }, `Emphasis ${s.emphasis ? 'off' : 'on'}`)}
        >Emphasis</button>
      </div>
    </details>

    <details>
      <summary>Clock</summary>

      <SelectField
        label="Clock Source"
        value={s.clockSource}
        options={CLOCK_SOURCES}
        disabled={!editable}
        onrequest={(source) =>
          request(
            { control: 'clock', source },
            `Clock source: ${CLOCK_SOURCES.find((o) => o.value === source)?.label}`,
          )}
      />
      <dl class="readout">
        <dt>Sample Rate</dt>
        <dd>{status ? formatRate(status.sampleRate) : '–'}</dd>
        <dt>Current</dt>
        <dd>{currentClock}</dd>
      </dl>
      <p class="note">
        The sample rate is set by the audio software, not here.
      </p>

      <h3>Input Status</h3>
      <dl class="readout" aria-label="Input status">
        {#each lockRows as [name, lock] (name)}
          <dt>{name}</dt>
          <dd class:locked={lock !== 'noLock'}>{LOCK_LABELS[lock]}</dd>
        {:else}
          <dt>–</dt>
          <dd>unavailable</dd>
        {/each}
      </dl>
    </details>

    <div class="stack flash">
      <button
        class="toggle wide"
        disabled
        title="Not available: FFADO erases and rewrites the settings flash from its own copy without checking it"
      >Store in Flash</button>
      <button
        class="toggle wide"
        disabled
        title="Not available: FFADO loads flash into its own copy without sending it to the unit"
      >Read Flash</button>
    </div>
    <p class="note">Flash store and read stay off until they can be made safe on this unit.</p>
  {/if}
</section>

<style>
  .device-settings {
    background: var(--panel);
    border-radius: 4px;
    padding: 6px 8px 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  h2 {
    margin: 0 0 2px;
    font-size: 11px;
    color: #e8f6ff;
  }
  h3 {
    margin: 6px 0 2px;
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-dim);
  }
  details {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding-top: 4px;
    border-top: 1px solid #1f2327;
  }
  summary {
    cursor: pointer;
    font-size: 10.5px;
    font-weight: 600;
    color: var(--text);
  }
  summary:focus-visible {
    outline: 1px solid var(--focus);
    outline-offset: 1px;
  }
  .note {
    margin: 0;
    font-size: 10px;
    line-height: 1.35;
    color: var(--text-dim);
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 3px;
  }
  .grid .span {
    grid-column: 1 / -1;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .flash {
    margin-top: 4px;
  }
  .pair {
    display: flex;
    gap: 3px;
  }
  .pair .toggle {
    flex: 1;
  }
  .confirm {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 6px;
    border: 1px solid var(--solo);
    border-radius: 4px;
    background: #2a2418;
  }
  .confirm p {
    margin: 0;
    font-size: 10.5px;
    line-height: 1.35;
  }
  .what {
    font-weight: 600;
    color: var(--text);
  }
  .warning {
    color: #f0d9a8;
  }
  .toggle.danger {
    color: #fff;
    background: var(--over);
  }
  .toggle.phantom[aria-pressed='true'] {
    color: #fff;
    background: var(--over);
  }
  .toggle.phantom[aria-pressed='mixed'] {
    color: var(--solo);
    border-style: dashed;
  }
  .check {
    display: flex;
    gap: 4px;
    align-items: flex-start;
    font-size: 10px;
    line-height: 1.35;
    color: var(--text-dim);
  }
  .check input {
    margin: 1px 0 0;
  }
  .readout {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 1px 6px;
    margin: 0;
    font-size: 10.5px;
  }
  .readout dt {
    color: var(--text-dim);
  }
  .readout dd {
    margin: 0;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
  }
  .readout dd.locked {
    color: var(--cue);
  }
</style>
