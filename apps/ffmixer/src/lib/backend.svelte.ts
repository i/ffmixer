// Bridge to the Rust session: reactive view state plus an action queue.
//
// Continuous controls (faders, knobs) can fire on every pointer move.
// Those actions are coalesced per control and flushed at most once per
// animation frame, in order, so the backend never sees a flood.

import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

import type { Action, ClockStatus, NameRow, Setting, ViewState } from './types'

export const session = $state<{ view: ViewState | null; error: string }>({
  view: null,
  error: '',
})

const queue = new Map<string, Action>()
let flushScheduled = false
let flushing = false
let discreteCounter = 0

/** Key under which repeated updates of one control replace each other. */
function coalesceKey(action: Action): string | null {
  switch (action.type) {
    case 'setSendGain':
    case 'setSendPan':
      return `${action.type}:${action.kind}:${action.channel}`
    case 'setOutputVolume':
    case 'setOutputBalance':
      return `${action.type}:${action.output}`
    default:
      return null
  }
}

export function send(action: Action): void {
  const key = coalesceKey(action) ?? `discrete:${discreteCounter++}`
  // Re-inserting moves a key to the end, keeping overall order intact.
  queue.delete(key)
  queue.set(key, action)
  scheduleFlush()
}

function scheduleFlush(): void {
  if (flushScheduled || flushing) return
  flushScheduled = true
  requestAnimationFrame(() => {
    flushScheduled = false
    void flush()
  })
}

async function flush(): Promise<void> {
  flushing = true
  try {
    while (queue.size > 0) {
      const [key, action] = queue.entries().next().value as [string, Action]
      queue.delete(key)
      try {
        session.view = await invoke<ViewState>('apply', { action })
        session.error = ''
      } catch (error) {
        session.error = String(error)
      }
    }
  } finally {
    flushing = false
  }
}

/** Latest clock and lock status from the unit; null if unavailable. */
export const deviceStatus = $state<{ value: ClockStatus | null }>({ value: null })

export async function connect(): Promise<void> {
  await listen<ViewState>('state', (event) => {
    session.view = event.payload
  })
  await listen<ClockStatus | null>('device-status', (event) => {
    deviceStatus.value = event.payload
  })
  session.view = await invoke<ViewState>('get_state')
}

/** Runs a command that replaces the whole view, e.g. going live. */
async function command(
  name: string,
  args: Record<string, unknown> = {},
): Promise<void> {
  try {
    session.view = await invoke<ViewState>(name, args)
    session.error = ''
  } catch (error) {
    session.error = String(error)
  }
}

/** Reads the unit again and starts sending, discarding unsent edits. */
export function reconnect(): Promise<void> {
  return command('reconnect')
}

/**
 * Renames a source strip (`index` = its first channel) or an output pair.
 * `null` restores the channel's own name. Display only; saved to disk.
 */
export function rename(row: NameRow, index: number, name: string | null): Promise<void> {
  return command('rename', { row, index, name })
}

/**
 * Asks what the user must confirm before `setting` is applied. Null means
 * it can be applied at once. The backend enforces this again.
 */
export function settingWarning(setting: Setting): Promise<string | null> {
  return invoke<string | null>('setting_warning', { setting })
}

/** Changes a device setting on the unit now (only while live). */
export function setDevice(setting: Setting, confirmed: boolean): Promise<void> {
  return command('apply', { action: { type: 'setDevice', setting, confirmed } })
}
