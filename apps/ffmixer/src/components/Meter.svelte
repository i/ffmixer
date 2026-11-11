<script lang="ts">
  import { onMount } from 'svelte'

  import { registerMeter } from '../lib/meters'
  import type { MeterRow } from '../lib/types'

  interface Props {
    row: MeterRow
    indices: number[]
  }

  let { row, indices }: Props = $props()
  let canvas: HTMLCanvasElement

  onMount(() => registerMeter({ canvas, row, indices }))
</script>

<!--
  The canvas is absolutely positioned so its pixel size (set while
  drawing) never feeds back into the layout.
-->
<div class="meter" style:width={`${indices.length * 7 + 2}px`}>
  <canvas bind:this={canvas} aria-hidden="true"></canvas>
</div>

<style>
  .meter {
    position: relative;
    flex-shrink: 0;
  }
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border-radius: 2px;
    box-shadow: inset 0 0 0 1px #0d0f11;
  }
</style>
