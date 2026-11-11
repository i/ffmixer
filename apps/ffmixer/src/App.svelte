<!-- Mixer window: three rows (inputs, playbacks, outputs) + Control Strip. -->
<script lang="ts">
  import { onMount } from 'svelte'

  import ControlStrip from './components/ControlStrip.svelte'
  import OutputStrip from './components/OutputStrip.svelte'
  import Row from './components/Row.svelte'
  import Strip from './components/Strip.svelte'
  import { connect, session } from './lib/backend.svelte'
  import { startMeters } from './lib/meters'
  import { viewOptions } from './lib/viewOptions.svelte'

  onMount(() => {
    connect().catch((error) => (session.error = String(error)))
    startMeters().catch((error) => (session.error = String(error)))
  })

  const view = $derived(session.view)
  /** Phones (PH 9/10) then Main (AN 1/2) sit at the right end, as in TotalMix. */
  const MONITOR_PAIRS = [4, 0]
  const monitorOutputs = $derived(
    view
      ? MONITOR_PAIRS.flatMap((pair) => view.outputs.filter((output) => output.pair === pair))
      : [],
  )
  const otherOutputs = $derived(
    view ? view.outputs.filter((output) => !MONITOR_PAIRS.includes(output.pair)) : [],
  )
  const selectedOutput = $derived(
    view?.outputs.find((output) => output.channel === view.selectedOutput),
  )
  const target = $derived(selectedOutput?.name ?? '')
  /** A send into a mono output has no pan. */
  const outputStereo = $derived(selectedOutput?.stereo ?? true)
</script>

{#if view}
  <main class="mixer" class:loading={view.mode === 'loading'}>
    <div class="rows">
      {#snippet inputs()}
        {#each view.inputs as strip (`${strip.channel}-${strip.stereo}`)}
          <Strip {strip} row="inputs" {target} {outputStereo} soloActive={view.anySoloed} />
        {/each}
      {/snippet}
      {#snippet playbacks()}
        {#each view.playbacks as strip (`${strip.channel}-${strip.stereo}`)}
          <Strip {strip} row="playbacks" {target} {outputStereo} soloActive={view.anySoloed} />
        {/each}
      {/snippet}

      {#if viewOptions.twoRow}
        <Row title="Hardware Inputs · Software Playback">
          {@render inputs()}
          <div class="divider"></div>
          {@render playbacks()}
        </Row>
      {:else}
        <Row title="Hardware Inputs">{@render inputs()}</Row>
        <Row title="Software Playback">{@render playbacks()}</Row>
      {/if}
      <Row title="Hardware Outputs">
        {#each otherOutputs as output (`${output.channel}-${output.stereo}`)}
          <OutputStrip {output} selected={output.channel === view.selectedOutput} />
        {/each}
        <div class="spacer"></div>
        {#each monitorOutputs as output (`${output.channel}-${output.stereo}`)}
          <OutputStrip {output} selected={output.channel === view.selectedOutput} />
        {/each}
      </Row>
    </div>
    <ControlStrip {view} />
  </main>
{:else}
  <p class="splash">{session.error || 'Starting…'}</p>
{/if}

<style>
  .mixer {
    display: flex;
    height: 100vh;
    width: 100vw;
  }
  .mixer.loading .rows {
    opacity: 0.5;
    pointer-events: none;
  }
  .rows {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px;
    background: #16181b;
  }
  .spacer {
    flex: 1;
    min-width: 12px;
  }
  .divider {
    width: 4px;
    flex-shrink: 0;
    background: #15171a;
  }
  .splash {
    margin: 40vh auto;
    text-align: center;
    color: var(--text-dim);
  }
</style>
