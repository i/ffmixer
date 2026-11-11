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
  const target = $derived(view ? view.outputs[view.selectedOutput].name : '')
</script>

{#if view}
  <main class="mixer" class:loading={view.mode === 'loading'}>
    <div class="rows">
      {#snippet inputs()}
        {#each view.inputs as strip (`${strip.channel}-${strip.stereo}`)}
          <Strip {strip} row="inputs" {target} soloActive={view.anySoloed} />
        {/each}
      {/snippet}
      {#snippet playbacks()}
        {#each view.playbacks as strip (`${strip.channel}-${strip.stereo}`)}
          <Strip {strip} row="playbacks" {target} soloActive={view.anySoloed} />
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
        {#each view.outputs as output (output.pair)}
          <OutputStrip {output} selected={output.pair === view.selectedOutput} />
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
