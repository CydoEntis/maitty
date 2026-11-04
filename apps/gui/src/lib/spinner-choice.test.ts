import { afterEach, describe, expect, it, vi } from 'vitest'
import { chosenSpinner, isSpinnerStyle, isWebSpinner, setSpinnerStyle, spinnerDefinition, spinnerFrames, subscribeSpinnerStyle } from './spinner-clock'

describe('spinner choice', () => {
  afterEach(() => { setSpinnerStyle('braille') })

  it('tells TUI spinners from web ones and rejects unknown names', () => {
    expect(isWebSpinner('circle')).toBe(true)
    expect(isWebSpinner('braille')).toBe(false)
    expect(isSpinnerStyle('ping')).toBe(true)
    expect(isSpinnerStyle('moon')).toBe(false)
  })

  it('keeps the Braille frames for text-only places while a web spinner is chosen', () => {
    setSpinnerStyle('equalizer')

    expect(chosenSpinner()).toBe('equalizer')
    expect(spinnerFrames()).toEqual(spinnerDefinition('braille').frames)
  })

  it('tells a subscriber when the choice changes, and only then', () => {
    const listener = vi.fn()
    const unsubscribe = subscribeSpinnerStyle(listener)

    setSpinnerStyle('dots')
    setSpinnerStyle('dots')
    unsubscribe()
    setSpinnerStyle('spokes')

    expect(listener).toHaveBeenCalledTimes(1)
  })
})
