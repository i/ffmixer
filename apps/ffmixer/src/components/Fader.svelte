<!--
  TotalMix-style fader (manual §25.3, §25.8). Single clicks never change
  the level: dragging is relative, so grabbing the thumb or the track
  never jumps, and nothing changes until the pointer has moved a few
  pixels. Shift while dragging moves finely. Double-click sets 0 dB.
  Wheel ±0.5 dB (Shift ±0.1). Keyboard: arrows ±0.5 dB (Shift ±0.1),
  Page Up/Down ±6 dB, Home 0 dB, End −∞.

  `unityShortcuts={false}` disables double-click and Home, for faders
  where a jump to 0 dB is dangerous (the main monitor outputs).
-->
<script lang="ts">
  import { FADER_MAX_DB, FADER_TICKS, clamp, dbToFader, faderToDb, formatGain } from '../lib/scale'

  interface Props {
    db: number | null
    label: string
    onchange: (db: number | null) => void
    /** Allow double-click and Home to jump to 0 dB. */
    unityShortcuts?: boolean
  }

  let { db, label, onchange, unityShortcuts = true }: Props = $props()

  const THUMB_HEIGHT = 22
  const FINE_FACTOR = 0.125
  /** Pointer travel (px) before a press counts as a drag. */
  const DRAG_THRESHOLD = 4

  let track: HTMLDivElement
  let dragging = $state(false)
  let pressed = false
  let local = $state<number | null>(null)
  let dragStartY = 0
  let dragStartPosition = 0

  /** While dragging, the fader follows the pointer, not the backend. */
  const shown = $derived(dragging ? local : db)
  const position = $derived(dbToFader(shown))

  function setDb(next: number | null): void {
    const clamped = next === null ? null : clamp(next, -90, FADER_MAX_DB)
    local = clamped
    onchange(clamped)
  }

  function stepDb(delta: number): void {
    const current = shown ?? -90
    const next = current + delta
    setDb(next <= -90 ? null : next)
  }

  function travel(): number {
    return Math.max(1, track.clientHeight - THUMB_HEIGHT)
  }

  function onpointerdown(event: PointerEvent): void {
    if (event.button !== 0) return
    event.preventDefault()
    track.focus()
    track.setPointerCapture(event.pointerId)
    pressed = true
    dragStartY = event.clientY
  }

  function onpointermove(event: PointerEvent): void {
    if (!pressed) return
    if (!dragging) {
      if (Math.abs(event.clientY - dragStartY) < DRAG_THRESHOLD) return
      // Start measuring from here so the threshold itself adds no jump.
      // Read the level before `dragging` flips: `shown` then switches to
      // `local`, which is stale (null on the first drag).
      const start = db
      dragging = true
      local = start
      dragStartY = event.clientY
      dragStartPosition = dbToFader(start)
      return
    }
    const factor = event.shiftKey ? FINE_FACTOR : 1
    const moved = ((dragStartY - event.clientY) / travel()) * factor
    setDb(faderToDb(clamp(dragStartPosition + moved, 0, 1)))
  }

  function endDrag(event: PointerEvent): void {
    if (!pressed) return
    track.releasePointerCapture(event.pointerId)
    pressed = false
    dragging = false
  }

  function onwheel(event: WheelEvent): void {
    event.preventDefault()
    const step = event.shiftKey ? 0.1 : 0.5
    stepDb(event.deltaY < 0 ? step : -step)
  }

  function onkeydown(event: KeyboardEvent): void {
    const fine = event.shiftKey ? 0.1 : 0.5
    const handled: Record<string, () => void> = {
      ArrowUp: () => stepDb(fine),
      ArrowDown: () => stepDb(-fine),
      PageUp: () => stepDb(6),
      PageDown: () => stepDb(-6),
      Home: () => {
        if (unityShortcuts) setDb(0)
      },
      End: () => setDb(null),
    }
    const action = handled[event.key]
    if (action) {
      event.preventDefault()
      action()
    }
  }
</script>

<div
  class="fader"
  bind:this={track}
  role="slider"
  tabindex="0"
  aria-label={label}
  aria-valuemin={-90}
  aria-valuemax={FADER_MAX_DB}
  aria-valuenow={shown ?? -90}
  aria-valuetext={`${formatGain(shown)} dB`}
  {onpointerdown}
  {onpointermove}
  onpointerup={endDrag}
  onpointercancel={endDrag}
  {onwheel}
  ondblclick={() => {
    if (unityShortcuts) setDb(0)
  }}
  {onkeydown}
>
  <div class="slot"></div>
  {#each FADER_TICKS as tick (tick)}
    <div
      class="tick"
      class:unity={tick === 0}
      style:bottom={`calc(${dbToFader(tick)} * (100% - ${THUMB_HEIGHT}px) + ${THUMB_HEIGHT / 2}px)`}
    ></div>
  {/each}
  <div
    class="thumb"
    class:active={dragging}
    style:bottom={`calc(${position} * (100% - ${THUMB_HEIGHT}px))`}
    style:height={`${THUMB_HEIGHT}px`}
  ></div>
</div>

<style>
  .fader {
    position: relative;
    width: 22px;
    cursor: ns-resize;
    touch-action: none;
    outline: none;
    border-radius: 3px;
  }
  .fader:focus-visible {
    box-shadow: 0 0 0 1px var(--focus);
  }
  .slot {
    position: absolute;
    left: 50%;
    top: 4px;
    bottom: 4px;
    width: 4px;
    transform: translateX(-50%);
    background: #15171a;
    border-radius: 2px;
    box-shadow: inset 0 1px 1px #000;
  }
  .tick {
    position: absolute;
    right: 0;
    width: 4px;
    height: 1px;
    background: #6b737c;
  }
  .tick.unity {
    width: 7px;
    background: #aab2bb;
  }
  .thumb {
    position: absolute;
    left: 1px;
    right: 1px;
    border-radius: 2px;
    background: repeating-linear-gradient(
        to bottom,
        transparent 0 3px,
        rgba(0, 0, 0, 0.22) 3px 4px
      ),
      linear-gradient(to bottom, #e9ecef, #9aa2aa);
    box-shadow: 0 2px 3px rgba(0, 0, 0, 0.6);
  }
  .thumb::after {
    content: '';
    position: absolute;
    left: 2px;
    right: 2px;
    top: 50%;
    height: 2px;
    margin-top: -1px;
    background: #3a8fd9;
  }
  .thumb.active {
    background: linear-gradient(to bottom, #fff, #b7bec5);
  }
</style>
