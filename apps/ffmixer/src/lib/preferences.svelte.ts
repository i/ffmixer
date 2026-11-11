// Preferences that outlast a restart, kept in the webview's local storage.

const HIDE_WARNINGS = 'ffmixer.hideWarnings'

export const preferences = $state({
  /**
   * Apply risky device changes (48 V on, levels, drive, limiter, clock)
   * without the confirmation prompt. Off by default.
   */
  hideWarnings: localStorage.getItem(HIDE_WARNINGS) === '1',
})

export function setHideWarnings(value: boolean): void {
  preferences.hideWarnings = value
  localStorage.setItem(HIDE_WARNINGS, value ? '1' : '0')
}
