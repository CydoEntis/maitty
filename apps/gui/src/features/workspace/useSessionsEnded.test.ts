import { act, renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { useSessionsEnded } from './useSessionsEnded'

const listing = (...ids: string[]) => new Map(ids.map((id) => [id, true]))

describe('the count of watched sessions that ended', () => {
  it('goes up when a watched session was listed and then is gone', () => {
    const hook = renderHook(({ listed }) => useSessionsEnded(listed), { initialProps: { listed: listing('a') } })
    act(() => { hook.result.current.watch('b') })
    hook.rerender({ listed: listing('a', 'b') })
    expect(hook.result.current.ended).toBe(0)
    hook.rerender({ listed: listing('a') })
    expect(hook.result.current.ended).toBe(1)
    hook.rerender({ listed: listing('a') })
    expect(hook.result.current.ended).toBe(1)
  })

  it('does not take a session the list has not shown yet for one that ended', () => {
    const hook = renderHook(({ listed }) => useSessionsEnded(listed), { initialProps: { listed: listing('a') } })
    act(() => { hook.result.current.watch('b') })
    hook.rerender({ listed: listing('a') })
    expect(hook.result.current.ended).toBe(0)
  })

  it('ignores a session nobody asked to watch', () => {
    const hook = renderHook(({ listed }) => useSessionsEnded(listed), { initialProps: { listed: listing('a', 'b') } })
    hook.rerender({ listed: listing('a') })
    expect(hook.result.current.ended).toBe(0)
  })
})
