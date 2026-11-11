// Labels for the device settings; names follow RME's Fireface Settings
// dialog (manual ch. 7).

import type {
  Adat2Output,
  ClockSource,
  InputChannel,
  InputLevel,
  InputSource,
  Lock,
  Mic,
  OutputLevel,
  SpdifFormat,
  SpdifInput,
  SyncSource,
} from './types'

export interface Option<T extends string> {
  value: T
  label: string
}

export const INPUT_CHANNELS: ReadonlyArray<{ channel: InputChannel; label: string }> = [
  { channel: 'ch1', label: 'Input 1' },
  { channel: 'ch7', label: 'Input 7' },
  { channel: 'ch8', label: 'Input 8' },
]

export const INPUT_SOURCES: ReadonlyArray<Option<InputSource>> = [
  { value: 'front', label: 'Front' },
  { value: 'rear', label: 'Rear' },
  { value: 'frontAndRear', label: 'Front + Rear' },
]

export const INPUT_LEVELS: ReadonlyArray<Option<InputLevel>> = [
  { value: 'loGain', label: 'Lo Gain' },
  { value: 'minus10dBV', label: '-10 dBV' },
  { value: 'plus4dBu', label: '+4 dBu' },
]

export const OUTPUT_LEVELS: ReadonlyArray<Option<OutputLevel>> = [
  { value: 'minus10dBV', label: '-10 dBV' },
  { value: 'plus4dBu', label: '+4 dBu' },
  { value: 'hiGain', label: 'Hi Gain' },
]

export const MICS: ReadonlyArray<{ mic: Mic; label: string }> = [
  { mic: 'mic7', label: 'Mic 7' },
  { mic: 'mic8', label: 'Mic 8' },
  { mic: 'mic9', label: 'Mic 9' },
  { mic: 'mic10', label: 'Mic 10' },
]

export const SPDIF_INPUTS: ReadonlyArray<Option<SpdifInput>> = [
  { value: 'coax', label: 'Coaxial' },
  { value: 'optical', label: 'Optical' },
]

export const ADAT2_OUTPUTS: ReadonlyArray<Option<Adat2Output>> = [
  { value: 'adat', label: 'ADAT' },
  { value: 'spdif', label: 'SPDIF' },
]

export const SPDIF_FORMATS: ReadonlyArray<Option<SpdifFormat>> = [
  { value: 'consumer', label: 'Consumer' },
  { value: 'professional', label: 'Professional' },
]

export const CLOCK_SOURCES: ReadonlyArray<Option<ClockSource>> = [
  { value: 'internal', label: 'Internal' },
  { value: 'word', label: 'Word Clock' },
  { value: 'adat1', label: 'ADAT 1' },
  { value: 'adat2', label: 'ADAT 2' },
  { value: 'spdif', label: 'SPDIF' },
]

export const SYNC_SOURCE_LABELS: Record<SyncSource, string> = {
  adat1: 'ADAT 1',
  adat2: 'ADAT 2',
  spdif: 'SPDIF',
  word: 'Word Clock',
  tco: 'TCO',
}

export const LOCK_LABELS: Record<Lock, string> = {
  noLock: 'No Lock',
  locked: 'Lock',
  synced: 'Sync',
}

/** 48000 → "48 kHz", 44100 → "44.1 kHz", 0 → "–". */
export function formatRate(hertz: number): string {
  if (hertz <= 0) return '–'
  return `${Number((hertz / 1000).toFixed(1))} kHz`
}
