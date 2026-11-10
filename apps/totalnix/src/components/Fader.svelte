<!--
  TotalMix-style fader (manual §25.3, §25.8):
  drag or mouse wheel to move; Shift for fine control; double-click or
  Ctrl+click toggles 0 dB and −∞. Keyboard: arrows ±0.5 dB (Shift ±0.1),
  Page Up/Down ±6 dB, Home 0 dB, End −∞.
-->
<script lang="ts">
  import { FADER_MAX_DB, FADER_TICKS, clamp, dbToFader, faderToDb, formatGain } from '../lib/scale'

  interface Props {
    db: number | null
    label: string
    onchange: (db: number | null) => void
  }

  let { db, label, onchange }: Props = $props()

  const THUMB_HEIGHT = 22
  const FINE_FACTOR = 0.125

  let track: HTMLDivElement
  let dragging = $state(false)
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

  function toggleUnity(): void {
    setDb(shown !== null && Math.abs(shown) < 0.05 ? null : 0)
  }

  function travel(): number {
    return Math.max(1, track.clientHeight - THUMB_HEIGHT)
  }

  function onpointerdown(event: PointerEvent): void {
    if (event.button !== 0) return
    if (event.ctrlKey) {
      toggleUnity()
      return
    }
    event.preventDefault()
    track.setPointerCapture(event.pointerId)
    const rect = track.getBoundingClientRect()
    const thumbTop = rect.top + (1 - position) * travel()
    const onThumb = event.clientY >= thumbTop && event.clientY <= thumbTop + THUMB_HEIGHT
    dragging = true
    local = shown
    if (!onThumb) {
      // Clicking the track jumps the thumb there, then drags from it.
      const jumped = 1 - (event.clientY - rect.top - THUMB_HEIGHT / 2) / travel()
      setDb(faderToDb(clamp(jumped, 0, 1)))
    }
    dragStartY = event.clientY
    dragStartPosition = dbToFader(local)
  }

  function onpointermove(event: PointerEvent): void {
    if (!dragging) return
    const factor = event.shiftKey ? FINE_FACTOR : 1
    const moved = ((dragStartY - event.clientY) / travel()) * factor
    setDb(faderToDb(clamp(dragStartPosition + moved, 0, 1)))
  }

  function endDrag(event: PointerEvent): void {
    if (!dragging) return
    track.releasePointerCapture(event.pointerId)
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
      Home: () => setDb(0),
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
  ondblclick={toggleUnity}
  {onwheel}
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
