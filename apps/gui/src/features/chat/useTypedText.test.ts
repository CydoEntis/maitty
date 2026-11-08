import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useTypedText } from './useTypedText'

beforeEach(() => { vi.useFakeTimers() })
afterEach(() => { vi.useRealTimers() })

describe('typed text', () => {
  it('shows what was there at once, then types toward what arrives, ending on all of it', () => {
    const { result, rerender } = renderHook(({ text }) => useTypedText(text, true), { initialProps: { text: 'Hello' } })
    expect(result.current).toBe('Hello')

    rerender({ text: 'Hello, and welcome to the new chat' })
    expect(result.current).toBe('Hello')
    act(() => { vi.advanceTimersByTime(24) })
    expect(result.current.length).toBeGreaterThan('Hello'.length)
    expect(result.current.length).toBeLessThan('Hello, and welcome to the new chat'.length)
    act(() => { vi.advanceTimersByTime(2000) })
    expect(result.current).toBe('Hello, and welcome to the new chat')
  })

  it('shows everything at once when it is off', () => {
    const { result, rerender } = renderHook(({ text }) => useTypedText(text, false), { initialProps: { text: 'a' } })
    rerender({ text: 'a much longer reply' })
    expect(result.current).toBe('a much longer reply')
  })

  it('never shows more than has arrived when the text shrinks', () => {
    const { result, rerender } = renderHook(({ text }) => useTypedText(text, true), { initialProps: { text: 'abc **bo' } })
    rerender({ text: 'abc' })
    expect(result.current).toBe('abc')
  })
})
