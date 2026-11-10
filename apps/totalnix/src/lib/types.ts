// Mirrors `src-tauri/src/view.rs`, `session.rs` and `meters.rs`.

export type SourceKind = 'input' | 'playback'

export type Mode = 'loading' | 'hardware' | 'demo'

export interface StripView {
  kind: SourceKind
  channel: number
  stereo: boolean
  name: string
  /** Send level into the selected submix; null is −∞. */
  gainDb: number | null
  pan: number
  muted: boolean
  soloed: boolean
  routed: boolean
  meters: number[]
}

export interface OutputView {
  pair: number
  name: string
  volumeDb: number | null
  balance: number
  muted: boolean
  meters: [number, number]
}

export interface ViewState {
  mode: Mode
  device: string
  status: string
  selectedOutput: number
  inputs: StripView[]
  playbacks: StripView[]
  outputs: OutputView[]
  anyMuted: boolean
  anySoloed: boolean
  pendingChanges: number | null
  warnings: string[]
}

export type Action =
  | { type: 'selectOutput'; output: number }
  | { type: 'setSendGain'; kind: SourceKind; channel: number; db: number | null }
  | { type: 'setSendPan'; kind: SourceKind; channel: number; pan: number }
  | { type: 'setMuted'; kind: SourceKind; channel: number; value: boolean }
  | { type: 'setSoloed'; kind: SourceKind; channel: number; value: boolean }
  | { type: 'setStereo'; kind: SourceKind; pair: number; value: boolean }
  | { type: 'setOutputVolume'; output: number; db: number | null }
  | { type: 'setOutputBalance'; output: number; pan: number }
  | { type: 'setOutputMuted'; output: number; value: boolean }
  | { type: 'toggleMuteMaster' }
  | { type: 'toggleSoloMaster' }

export type MeterRow = 'inputs' | 'playbacks' | 'outputs'

/** dBFS per channel; −150 means silence. */
export type MeterEvent = Record<MeterRow, number[]>
