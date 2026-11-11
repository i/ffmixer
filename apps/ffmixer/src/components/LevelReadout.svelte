<!-- Numeric level per channel, refreshed twice a second (manual §25.3). -->
<script lang="ts">
  import { onMount } from 'svelte'

  import { readout } from '../lib/meters'
  import { formatLevel } from '../lib/scale'
  import { viewOptions } from '../lib/viewOptions.svelte'
  import type { MeterRow } from '../lib/types'

  interface Props {
    row: MeterRow
    indices: number[]
  }

  let { row, indices }: Props = $props()
  let values = $state<number[]>([])

  onMount(() => {
    const refresh = () => {
      values = indices.map((index) => readout(row, index, viewOptions.rms))
    }
    refresh()
    const timer = setInterval(refresh, 500)
    return () => clearInterval(timer)
  })
</script>

<div class="readout" class:narrow={indices.length === 1}>
  {#each values as value, slot (slot)}
    <span class:over={value >= 0}>{formatLevel(value)}</span>
  {/each}
</div>

<style>
  .readout {
    display: flex;
    justify-content: center;
    gap: 3px;
    height: 14px;
    font-size: 9.5px;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .over {
    color: var(--over);
    font-weight: 600;
  }
</style>
