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
  /** The unit's device settings; null if FFADO would not report them. */
  deviceSettings: DeviceSettings | null
  warnings: string[]
}

// Device settings (`ffado-dbus/src/settings.rs`).

export type Mic = 'mic7' | 'mic8' | 'mic9' | 'mic10'
export type InputChannel = 'ch1' | 'ch7' | 'ch8'
export type InputSource = 'front' | 'rear' | 'frontAndRear'
export type InputLevel = 'loGain' | 'minus10dBV' | 'plus4dBu'
export type OutputLevel = 'minus10dBV' | 'plus4dBu' | 'hiGain'
export type SpdifInput = 'coax' | 'optical'
export type Adat2Output = 'adat' | 'spdif'
export type SpdifFormat = 'consumer' | 'professional'
export type ClockSource = 'internal' | 'word' | 'adat1' | 'adat2' | 'spdif'

export type Setting =
  | { control: 'inputSource'; channel: InputChannel; source: InputSource }
  | { control: 'inputLevel'; level: InputLevel }
  | { control: 'outputLevel'; level: OutputLevel }
  | { control: 'phantom'; mic: Mic; on: boolean }
  | { control: 'drive'; on: boolean }
  | { control: 'limiter'; on: boolean }
  | { control: 'speakerEmulation'; on: boolean }
  | { control: 'spdifInput'; input: SpdifInput }
  | { control: 'adat2Output'; output: Adat2Output }
  | { control: 'spdifFormat'; format: SpdifFormat }
  | { control: 'emphasis'; on: boolean }
  | { control: 'clock'; source: ClockSource }

export interface DeviceSettings {
  inputSources: Record<InputChannel, InputSource>
  inputLevel: InputLevel
  outputLevel: OutputLevel
  /** Mic 7–10; null is unknown (FFADO cannot report Mic 10). */
  phantom: (boolean | null)[]
  instrument: { drive: boolean; limiter: boolean; speakerEmulation: boolean }
  spdifInput: SpdifInput
  adat2Output: Adat2Output
  spdifFormat: SpdifFormat
  emphasis: boolean
  /** null if the unit is set to the TCO. */
  clockSource: ClockSource | null
}

export type Lock = 'noLock' | 'locked' | 'synced'
export type SyncSource = 'adat1' | 'adat2' | 'spdif' | 'word' | 'tco'

/** Read-only clock and input lock state, sent once a second. */
export interface ClockStatus {
  sampleRate: number
  master: boolean
  autosyncSource: SyncSource | null
  autosyncRate: number
  spdifRate: number
  word: Lock
  spdif: Lock
  adat1: Lock
  adat2: Lock
  tco: Lock | null
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
  | { type: 'setDevice'; setting: Setting; confirmed: boolean }

export type MeterRow = 'inputs' | 'playbacks' | 'outputs'

/** dBFS per channel; −150 means silence. */
export type MeterEvent = Record<MeterRow, number[]>
