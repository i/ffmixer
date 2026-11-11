// Fader and meter scales, and value formatting.

/** Fader travel (0 = bottom, 1 = top) for each marked dB value. */
const FADER_POINTS: ReadonlyArray<readonly [number, number]> = [
  [6, 1],
  [0, 0.84],
  [-5, 0.72],
  [-10, 0.61],
  [-20, 0.43],
  [-30, 0.3],
  [-40, 0.19],
  [-60, 0.07],
  [-90, 0.02],
]

export const FADER_MAX_DB = 6
export const FADER_TICKS = [6, 0, -5, -10, -20, -30, -40, -60]

/** Meter travel for each marked dBFS value. */
const METER_POINTS: ReadonlyArray<readonly [number, number]> = [
  [0, 1],
  [-6, 0.88],
  [-12, 0.76],
  [-18, 0.64],
  [-24, 0.53],
  [-30, 0.43],
  [-40, 0.3],
  [-50, 0.19],
  [-60, 0.09],
  [-90, 0],
]

function interpolate(
  points: ReadonlyArray<readonly [number, number]>,
  value: number,
  from: 0 | 1,
  to: 0 | 1,
): number {
  const first = points[0]
  const last = points[points.length - 1]
  const descending = first[from] > last[from]
  const clampHigh = descending ? first : last
  const clampLow = descending ? last : first
  if (value >= clampHigh[from]) return clampHigh[to]
  if (value <= clampLow[from]) return clampLow[to]
  for (let i = 0; i < points.length - 1; i++) {
    const a = points[i]
    const b = points[i + 1]
    const low = Math.min(a[from], b[from])
    const high = Math.max(a[from], b[from])
    if (value >= low && value <= high) {
      const t = (value - a[from]) / (b[from] - a[from])
      return a[to] + t * (b[to] - a[to])
    }
  }
  return clampLow[to]
}

/** dB (null = −∞) → fader position 0..1. */
export function dbToFader(db: number | null): number {
  return db === null ? 0 : interpolate(FADER_POINTS, db, 0, 1)
}

/** Fader position 0..1 → dB (null = −∞ at the very bottom). */
export function faderToDb(position: number): number | null {
  if (position <= 0.005) return null
  const lowest = FADER_POINTS[FADER_POINTS.length - 1]
  if (position < lowest[1]) return lowest[0]
  return interpolate(FADER_POINTS, position, 1, 0)
}

/** dBFS → meter position 0..1. */
export function dbfsToMeter(dbfs: number): number {
  return interpolate(METER_POINTS, dbfs, 0, 1)
}

export function formatGain(db: number | null): string {
  if (db === null || db < -89.95) return '-∞'
  const rounded = Math.round(db * 10) / 10
  return rounded === 0 ? '0.0' : rounded.toFixed(1)
}

export function formatLevel(dbfs: number): string {
  if (dbfs <= -90) return '-∞'
  if (dbfs >= 0) return 'OVR'
  return dbfs.toFixed(1)
}

/** Pan −1..1 shown TotalMix-style as −100..100 (0 = centre). */
export function formatPan(pan: number): string {
  const value = Math.round(pan * 100)
  if (value === 0) return '0'
  return value < 0 ? `L${-value}` : `R${value}`
}

export function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value))
}
