// Bridge to the Rust session: reactive view state plus an action queue.
//
// Continuous controls (faders, knobs) can fire on every pointer move.
// Those actions are coalesced per control and flushed at most once per
// animation frame, in order, so the backend never sees a flood.

import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

import type { Action, ViewState } from './types'

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

export async function connect(): Promise<void> {
  await listen<ViewState>('state', (event) => {
    session.view = event.payload
  })
  session.view = await invoke<ViewState>('get_state')
}
