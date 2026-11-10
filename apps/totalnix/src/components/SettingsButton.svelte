<!-- Tool symbol that opens a channel's Settings panel (manual §25.3). -->
<script lang="ts">
  interface Props {
    open: boolean
    ontoggle: () => void
  }

  let { open, ontoggle }: Props = $props()
</script>

<button class="icon" class:open title="Channel settings" aria-expanded={open} onclick={ontoggle}>
  <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
    <path
      fill="currentColor"
      d="M10.8 1a4.2 4.2 0 0 0-3.9 5.7L1.4 12.2a1.4 1.4 0 0 0 2 2l5.5-5.5A4.2 4.2 0 0 0 14.6 5l-2.4 2.4-2.3-.6-.6-2.3L11.7 2A4.2 4.2 0 0 0 10.8 1z"
    />
  </svg>
</button>

<style>
  .icon {
    align-self: flex-end;
  }
</style>
