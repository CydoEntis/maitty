import { describe, expect, it } from 'vitest'
import { isLinuxWebView } from './gpu'

const WEBKITGTK = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/60.5 Safari/605.1.15'
const WEBKITGTK_BUNDLED = 'Mozilla/5.0 (X11; Ubuntu; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/60.5 Safari/605.1.15'
const MACOS = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15'
const WINDOWS = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36'

describe('telling the Linux web view from the others', () => {
  it('knows WebKitGTK, with or without the distribution in its user agent', () => {
    expect(isLinuxWebView(WEBKITGTK)).toBe(true)
    expect(isLinuxWebView(WEBKITGTK_BUNDLED)).toBe(true)
  })

  it('is not macOS, Windows or the test environment', () => {
    expect(isLinuxWebView(MACOS)).toBe(false)
    expect(isLinuxWebView(WINDOWS)).toBe(false)
    expect(isLinuxWebView()).toBe(false)
  })
})
