// Preferences that outlast a restart, kept in the webview's local storage.

const SKIP_PHANTOM_WARNING = 'ffmixer.skipPhantomWarning'

export const preferences = $state({
  /** Switch 48 V on without the confirmation prompt. Off by default. */
  skipPhantomWarning: localStorage.getItem(SKIP_PHANTOM_WARNING) === '1',
})

export function setSkipPhantomWarning(value: boolean): void {
  preferences.skipPhantomWarning = value
  localStorage.setItem(SKIP_PHANTOM_WARNING, value ? '1' : '0')
}
