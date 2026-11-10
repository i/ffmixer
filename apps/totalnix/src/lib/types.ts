// Mirrors `src-tauri/src/view.rs`, `session.rs`, `channel_names.rs` and `meters.rs`.

export type SourceKind = 'input' | 'playback'

/** Which row a renamed strip is in (`channel_names.rs`). */
export type NameRow = SourceKind | 'output'

export type Mode = 'loading' | 'hardware' | 'demo'

export interface StripView {
  kind: SourceKind
  channel: number
  stereo: boolean
  /** Custom name if set, else the channel's own name. */
  name: string
  defaultName: string
  /** Send level into the selected submix; null is −∞. */
  gainDb: number | null
  pan: number
  muted: boolean
  soloed: boolean
  routed: boolean
  /** Phase inverted, per channel of the strip (one entry if mono). */
  phase: boolean[]
  /** Stereo width, 1 = normal (stereo strips only). */
  width: number
  msProc: boolean
  meters: number[]
}

export interface OutputView {
  pair: number
  name: string
  defaultName: string
  volumeDb: number | null
  balance: number
  muted: boolean
  /** Phase L / Phase R. */
  phase: [boolean, boolean]
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
  /** True while edits are being sent to the unit. */
  live: boolean
  warnings: string[]
}

export type Action =
  | { type: 'selectOutput'; output: number }
  | { type: 'setSendGain'; kind: SourceKind; channel: number; db: number | null }
  | { type: 'setSendPan'; kind: SourceKind; channel: number; pan: number }
  | { type: 'setMuted'; kind: SourceKind; channel: number; value: boolean }
  | { type: 'setSoloed'; kind: SourceKind; channel: number; value: boolean }
  | { type: 'setStereo'; kind: SourceKind; pair: number; value: boolean }
  | { type: 'setPhase'; kind: SourceKind; channel: number; value: boolean }
  | { type: 'setWidth'; kind: SourceKind; pair: number; width: number }
  | { type: 'setMsProc'; kind: SourceKind; pair: number; value: boolean }
  | { type: 'setOutputPhase'; channel: number; value: boolean }
  | { type: 'setOutputVolume'; output: number; db: number | null }
  | { type: 'setOutputBalance'; output: number; pan: number }
  | { type: 'setOutputMuted'; output: number; value: boolean }
  | { type: 'toggleMuteMaster' }
  | { type: 'toggleSoloMaster' }

export type MeterRow = 'inputs' | 'playbacks' | 'outputs'

/** dBFS per channel; −150 means silence. */
export type MeterEvent = Record<MeterRow, number[]>
