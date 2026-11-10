<!-- One mixer row: strips scroll horizontally above a TotalMix-style label. -->
<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    title: string
    children: Snippet
  }

  let { title, children }: Props = $props()
</script>

<section class="row" aria-label={title}>
  <div class="strips">
    {@render children()}
  </div>
  <div class="label">{title}</div>
</section>

<style>
  .row {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    flex: 1;
    border-bottom: 2px solid #15171a;
  }
  .strips {
    flex: 1;
    min-height: 0;
    display: flex;
    overflow-x: auto;
    overflow-y: hidden;
  }
  .label {
    flex-shrink: 0;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
    background: var(--panel-dark);
    padding: 2px 8px;
  }
</style>
