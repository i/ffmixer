<!--
  Knob for pan, balance or width (manual §25.8), range −1 to +1: drag
  vertically or use the wheel (Shift speeds up); double-click or
  Ctrl+click resets (centre by default); Shift+click goes to −1;
  Shift+Ctrl+click to +1. Arrow keys step, Home resets.
-->
<script lang="ts">
  import { clamp, formatPan } from '../lib/scale'

  interface Props {
    value: number
    label: string
    onchange: (value: number) => void
    /** Text shown next to the knob. Defaults to pan style (L50, 0, R50). */
    format?: (value: number) => string
    /** Value for double-click, Ctrl+click and Home. */
    resetValue?: number
  }

  let { value, label, onchange, format = formatPan, resetValue = 0 }: Props = $props()

  const PIXELS_FOR_FULL_RANGE = 160
  const FAST_FACTOR = 4
  const SWEEP_DEGREES = 135

  let dragging = $state(false)
  let local = $state(0)
  let startY = 0
  let startValue = 0

  const shown = $derived(dragging ? local : value)

  function set(next: number): void {
    local = clamp(next, -1, 1)
    onchange(local)
  }

  function onpointerdown(event: PointerEvent): void {
    if (event.button !== 0) return
    if (event.shiftKey) {
      set(event.ctrlKey ? 1 : -1)
      return
    }
    if (event.ctrlKey) {
      set(resetValue)
      return
    }
    event.preventDefault()
    ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
    dragging = true
    local = value
    startY = event.clientY
    startValue = value
  }

  function onpointermove(event: PointerEvent): void {
    if (!dragging) return
    const factor = event.shiftKey ? FAST_FACTOR : 1
    set(startValue + ((startY - event.clientY) * 2 * factor) / PIXELS_FOR_FULL_RANGE)
  }

  function endDrag(event: PointerEvent): void {
    if (!dragging) return
    ;(event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId)
    dragging = false
  }

  function onwheel(event: WheelEvent): void {
    event.preventDefault()
    const step = event.shiftKey ? 0.1 : 0.02
    set(shown + (event.deltaY < 0 ? step : -step))
  }

  function onkeydown(event: KeyboardEvent): void {
    const step = event.shiftKey ? 0.1 : 0.02
    if (event.key === 'ArrowRight' || event.key === 'ArrowUp') set(shown + step)
    else if (event.key === 'ArrowLeft' || event.key === 'ArrowDown') set(shown - step)
    else if (event.key === 'Home') set(resetValue)
    else return
    event.preventDefault()
  }
</script>

<div class="knob-row">
  <div
    class="knob"
    role="slider"
    tabindex="0"
    aria-label={label}
    aria-valuemin={-100}
    aria-valuemax={100}
    aria-valuenow={Math.round(shown * 100)}
    aria-valuetext={format(shown)}
    {onpointerdown}
    {onpointermove}
    onpointerup={endDrag}
    onpointercancel={endDrag}
    ondblclick={() => set(resetValue)}
    {onwheel}
    {onkeydown}
  >
    <div class="cap" style:transform={`rotate(${shown * SWEEP_DEGREES}deg)`}>
      <div class="pointer"></div>
    </div>
  </div>
  <span class="value">{format(shown)}</span>
</div>

<style>
  .knob-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 3px;
    height: 30px;
  }
  .knob {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: #1a1d21;
    padding: 2px;
    cursor: ns-resize;
    touch-action: none;
    outline: none;
    flex-shrink: 0;
  }
  .knob:focus-visible {
    box-shadow: 0 0 0 1px var(--focus);
  }
  .cap {
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    background: radial-gradient(circle at 35% 30%, #8b939c, #454b52 70%);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.7);
  }
  .pointer {
    position: absolute;
    left: 50%;
    top: 2px;
    width: 2px;
    height: 9px;
    margin-left: -1px;
    background: var(--accent);
    border-radius: 1px;
  }
  .value {
    min-width: 24px;
    font-size: 10px;
    text-align: center;
    color: var(--accent);
    background: #1a1d21;
    border-radius: 3px;
    padding: 1px 2px;
    font-variant-numeric: tabular-nums;
  }
</style>
