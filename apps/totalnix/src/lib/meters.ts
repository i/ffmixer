// Meter ballistics and drawing, kept outside Svelte's reactivity: meter
// data arrives ~30 times a second and is drawn on canvases once per
// animation frame.
//
// The hardware reports a peak per reading. From that we derive what
// TotalMix shows: a yellow peak-hold line, a green "RMS" bar (here an
// energy average with a slow time constant) and a red over indicator.

import { listen } from '@tauri-apps/api/event'

import { dbfsToMeter } from './scale'
import type { MeterEvent, MeterRow } from './types'

const CHANNELS = 28
const PEAK_HOLD_MS = 1000
const PEAK_FALL_DB_PER_S = 26
const AVERAGE_TIME_CONSTANT_MS = 300
const OVER_DBFS = -0.1
const OVER_HOLD_MS = 2000
const READOUT_INTERVAL_MS = 500
export const SILENCE = -150

interface Channel {
  /** Latest peak from the hardware. */
  peak: number
  /** Displayed peak-hold line. */
  held: number
  heldAt: number
  /** Mean-square energy, linear. */
  energy: number
  overUntil: number
  /** Readout values, refreshed twice a second like TotalMix. */
  readoutPeak: number
  readoutAverage: number
  windowPeak: number
}

function silentChannel(): Channel {
  return {
    peak: SILENCE,
    held: SILENCE,
    heldAt: 0,
    energy: 0,
    overUntil: 0,
    readoutPeak: SILENCE,
    readoutAverage: SILENCE,
    windowPeak: SILENCE,
  }
}

const rows: Record<MeterRow, Channel[]> = {
  inputs: Array.from({ length: CHANNELS }, silentChannel),
  playbacks: Array.from({ length: CHANNELS }, silentChannel),
  outputs: Array.from({ length: CHANNELS }, silentChannel),
}

let lastUpdate = performance.now()
let lastReadout = 0

function toDb(energy: number): number {
  return energy <= 1e-15 ? SILENCE : 10 * Math.log10(energy)
}

function ingest(event: MeterEvent): void {
  const now = performance.now()
  const elapsed = now - lastUpdate
  lastUpdate = now
  const smoothing = 1 - Math.exp(-elapsed / AVERAGE_TIME_CONSTANT_MS)
  const refreshReadout = now - lastReadout >= READOUT_INTERVAL_MS
  if (refreshReadout) lastReadout = now

  for (const row of Object.keys(rows) as MeterRow[]) {
    const levels = event[row]
    rows[row].forEach((channel, index) => {
      const peak = levels[index] ?? SILENCE
      channel.peak = peak
      const amplitude = peak <= SILENCE ? 0 : 10 ** (peak / 20)
      channel.energy += (amplitude * amplitude - channel.energy) * smoothing
      if (peak >= channel.held) {
        channel.held = peak
        channel.heldAt = now
      }
      if (peak >= OVER_DBFS) channel.overUntil = now + OVER_HOLD_MS
      channel.windowPeak = Math.max(channel.windowPeak, peak)
      if (refreshReadout) {
        channel.readoutPeak = channel.windowPeak
        channel.readoutAverage = toDb(channel.energy)
        channel.windowPeak = SILENCE
      }
    })
  }
}

function decayHolds(now: number): void {
  for (const row of Object.values(rows)) {
    for (const channel of row) {
      const sinceHold = now - channel.heldAt - PEAK_HOLD_MS
      if (sinceHold > 0) {
        const fallen = channel.held - (PEAK_FALL_DB_PER_S * sinceHold) / 1000
        channel.held = Math.max(channel.peak, fallen, SILENCE)
        if (channel.held === channel.peak) channel.heldAt = now - PEAK_HOLD_MS
      }
    }
  }
}

export function readout(row: MeterRow, index: number, average: boolean): number {
  const channel = rows[row][index]
  return average ? channel.readoutAverage : channel.readoutPeak
}

interface Registration {
  canvas: HTMLCanvasElement
  row: MeterRow
  indices: number[]
}

const registrations = new Set<Registration>()

export function registerMeter(registration: Registration): () => void {
  registrations.add(registration)
  return () => registrations.delete(registration)
}

const COLOR_BACKGROUND = '#1b1e22'
const COLOR_BAR_LOW = '#5fd12b'
const COLOR_BAR_HIGH = '#d7e636'
const COLOR_PEAK = '#f2d33a'
const COLOR_OVER = '#e8392b'
const COLOR_OVER_IDLE = '#4a2826'
const OVER_HEIGHT = 6

function draw(now: number): void {
  for (const { canvas, row, indices } of registrations) {
    const context = canvas.getContext('2d')
    if (!context) continue
    const width = canvas.clientWidth
    const height = canvas.clientHeight
    const scale = window.devicePixelRatio || 1
    if (canvas.width !== Math.round(width * scale) || canvas.height !== Math.round(height * scale)) {
      canvas.width = Math.round(width * scale)
      canvas.height = Math.round(height * scale)
    }
    context.setTransform(scale, 0, 0, scale, 0, 0)
    context.fillStyle = COLOR_BACKGROUND
    context.fillRect(0, 0, width, height)

    const gap = 2
    const barWidth = (width - gap * (indices.length - 1)) / indices.length
    const travel = height - OVER_HEIGHT - 1
    const gradient = context.createLinearGradient(0, height, 0, OVER_HEIGHT)
    gradient.addColorStop(0, COLOR_BAR_LOW)
    gradient.addColorStop(0.75, COLOR_BAR_LOW)
    gradient.addColorStop(1, COLOR_BAR_HIGH)

    indices.forEach((index, slot) => {
      const channel = rows[row][index]
      const x = slot * (barWidth + gap)
      const bar = dbfsToMeter(toDb(channel.energy)) * travel
      context.fillStyle = gradient
      context.fillRect(x, height - bar, barWidth, bar)

      const held = dbfsToMeter(channel.held) * travel
      if (held > 1) {
        context.fillStyle = COLOR_PEAK
        context.fillRect(x, height - held - 1, barWidth, 2)
      }

      context.fillStyle = now < channel.overUntil ? COLOR_OVER : COLOR_OVER_IDLE
      context.fillRect(x, 0, barWidth, OVER_HEIGHT - 1)
    })
  }
}

function frame(now: number): void {
  decayHolds(now)
  draw(now)
  requestAnimationFrame(frame)
}

export async function startMeters(): Promise<void> {
  await listen<MeterEvent>('meters', (event) => ingest(event.payload))
  requestAnimationFrame(frame)
}
