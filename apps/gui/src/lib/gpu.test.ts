import { describe, expect, it } from 'vitest'
import { prefersPlainDrawing } from './gpu'

const WEBKITGTK = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/60.5 Safari/605.1.15'
const MACOS = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15'
const WINDOWS = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36'

describe('which web views start terminals on the DOM renderer', () => {
  it('chooses it for WebKitGTK, where WebGL showed typed letters late', () => {
    expect(prefersPlainDrawing(WEBKITGTK)).toBe(true)
  })

  it('keeps WebGL on macOS and Windows, and in the test environment', () => {
    expect(prefersPlainDrawing(MACOS)).toBe(false)
    expect(prefersPlainDrawing(WINDOWS)).toBe(false)
    expect(prefersPlainDrawing()).toBe(false)
  })
})
