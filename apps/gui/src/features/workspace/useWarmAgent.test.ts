import { renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useWarmAgent, WARM_AGAIN_MS, WARM_SETTLE_MS, type WarmSpec } from './useWarmAgent'

const SPEC: WarmSpec = { folder: '/code/app', agent: 'claude', model: 'default', reasoning: 'default', mode: 'default' }

type Props = { spec: WarmSpec | null; paused: boolean }

function mount(initial: Props) {
  const warm = vi.fn<(spec: WarmSpec | null) => void>()
  const hook = renderHook(({ spec, paused }: Props) => { useWarmAgent(warm, spec, paused) }, { initialProps: initial })
  return { warm, ...hook }
}

describe('keeping an agent warm for a new chat', () => {
  beforeEach(() => { vi.useFakeTimers() })
  afterEach(() => { vi.useRealTimers() })

  it('asks once the picks have come to rest, and again while the page stays open', () => {
    const { warm } = mount({ spec: SPEC, paused: false })
    expect(warm).not.toHaveBeenCalled()

    vi.advanceTimersByTime(WARM_SETTLE_MS)
    expect(warm).toHaveBeenCalledExactlyOnceWith(SPEC)

    vi.advanceTimersByTime(WARM_AGAIN_MS)
    expect(warm).toHaveBeenCalledTimes(2)
    expect(warm).toHaveBeenLastCalledWith(SPEC)
  })

  it('asks for the last pick only when picks follow each other quickly', () => {
    const { warm, rerender } = mount({ spec: SPEC, paused: false })
    vi.advanceTimersByTime(WARM_SETTLE_MS - 1)
    const opus = { ...SPEC, model: 'opus' }
    rerender({ spec: opus, paused: false })
    vi.advanceTimersByTime(WARM_SETTLE_MS)

    expect(warm).toHaveBeenCalledExactlyOnceWith(opus)
  })

  it('asks nothing while the chat is being started, so the start takes the agent that is warm', () => {
    const { warm, rerender } = mount({ spec: SPEC, paused: false })
    vi.advanceTimersByTime(WARM_SETTLE_MS)
    warm.mockClear()

    rerender({ spec: SPEC, paused: true })
    vi.advanceTimersByTime(WARM_AGAIN_MS * 2)

    expect(warm).not.toHaveBeenCalled()
  })

  it('says none is wanted when no agent can be started ahead, and when the page goes', () => {
    const { warm, rerender, unmount } = mount({ spec: SPEC, paused: false })
    vi.advanceTimersByTime(WARM_SETTLE_MS)
    warm.mockClear()

    rerender({ spec: null, paused: false })
    expect(warm).toHaveBeenCalledExactlyOnceWith(null)

    warm.mockClear()
    rerender({ spec: SPEC, paused: false })
    unmount()
    expect(warm).toHaveBeenCalledExactlyOnceWith(null)
    vi.advanceTimersByTime(WARM_AGAIN_MS)
    expect(warm).toHaveBeenCalledTimes(1)
  })
})
