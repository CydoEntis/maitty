import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { attachSpinner, setSpinnerStyle, SPINNER_TICK_MS, spinnerFrames } from './spinner-clock'

function focusedWindow(focused: boolean) {
  vi.spyOn(document, 'hasFocus').mockReturnValue(focused)
}

describe('spinner clock', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    focusedWindow(true)
    setSpinnerStyle('braille')
  })
  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('writes each frame into the element itself', () => {
    const element = document.createElement('span')
    const detach = attachSpinner(element)
    const frames = spinnerFrames()
    const first = element.textContent

    vi.advanceTimersByTime(SPINNER_TICK_MS)

    expect(frames).toContain(first)
    expect(element.textContent).not.toBe(first)
    expect(frames).toContain(element.textContent)
    detach()
  })

  it('keeps every spinner on the same frame', () => {
    const one = document.createElement('span')
    const two = document.createElement('span')
    const detachOne = attachSpinner(one)
    vi.advanceTimersByTime(SPINNER_TICK_MS * 3)
    const detachTwo = attachSpinner(two)

    expect(two.textContent).toBe(one.textContent)
    vi.advanceTimersByTime(SPINNER_TICK_MS)
    expect(two.textContent).toBe(one.textContent)
    detachOne()
    detachTwo()
  })

  it('keeps stepping while the window is visible with focus elsewhere', () => {
    const element = document.createElement('span')
    const detach = attachSpinner(element)

    focusedWindow(false)
    window.dispatchEvent(new Event('blur'))
    const before = element.textContent
    vi.advanceTimersByTime(SPINNER_TICK_MS * 5)
    expect(element.textContent).not.toBe(before)
    detach()
  })

  it('stops stepping while the window is hidden and resumes when it is shown', () => {
    const element = document.createElement('span')
    const detach = attachSpinner(element)
    const hidden = vi.spyOn(document, 'hidden', 'get').mockReturnValue(true)
    document.dispatchEvent(new Event('visibilitychange'))
    const held = element.textContent
    vi.advanceTimersByTime(SPINNER_TICK_MS * 5)
    expect(element.textContent).toBe(held)

    hidden.mockReturnValue(false)
    document.dispatchEvent(new Event('visibilitychange'))
    vi.advanceTimersByTime(SPINNER_TICK_MS)
    expect(element.textContent).not.toBe(held)
    detach()
  })

  it('stops the clock when the last spinner goes away', () => {
    const element = document.createElement('span')
    attachSpinner(element)()

    expect(vi.getTimerCount()).toBe(0)
  })

  it('marks a two-character style as wide and switches the frames at once', () => {
    const element = document.createElement('span')
    const detach = attachSpinner(element)

    setSpinnerStyle('bars')

    expect(element.classList.contains('is-wide')).toBe(true)
    expect(element.textContent.length).toBe(2)
    setSpinnerStyle('braille')
    expect(element.classList.contains('is-wide')).toBe(false)
    detach()
  })
})
