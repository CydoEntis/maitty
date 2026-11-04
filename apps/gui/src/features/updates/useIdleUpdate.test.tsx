import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { IDLE_INPUT_MS } from './idle'
import { useIdleUpdate } from './useIdleUpdate'

type Props = Parameters<typeof useIdleUpdate>[0]

let clock = 0
const now = () => clock

function setup(overrides: Partial<Props> = {}) {
  const onDownload = vi.fn()
  const onInstall = vi.fn()
  const initialProps: Props = { enabled: true, agentBusy: false, status: 'ready', offeredIsSkipped: false, onDownload, onInstall, now, ...overrides }
  const view = renderHook((props: Props) => { useIdleUpdate(props) }, { initialProps })
  return { onDownload, onInstall, initialProps, ...view }
}

function pass(ms: number) {
  clock += ms
  act(() => { vi.advanceTimersByTime(ms) })
}

describe('idle update', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    clock = 1_000
    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
  })
  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('installs a fetched update once the window has been away and untouched for five minutes', () => {
    const { onInstall } = setup()
    expect(onInstall).not.toHaveBeenCalled()

    pass(IDLE_INPUT_MS - 30_000)
    expect(onInstall).not.toHaveBeenCalled()
    pass(30_000)
    expect(onInstall).toHaveBeenCalledTimes(1)
  })

  it('starts the five minutes again at every key', () => {
    const { onInstall } = setup()
    pass(IDLE_INPUT_MS - 60_000)
    act(() => { window.dispatchEvent(new KeyboardEvent('keydown', { key: 'a' })) })

    pass(IDLE_INPUT_MS - 30_000)
    expect(onInstall).not.toHaveBeenCalled()
    pass(30_000)
    expect(onInstall).toHaveBeenCalledTimes(1)
  })

  it('waits while the window is in front, and installs when it goes away', () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(true)
    const { onInstall } = setup()
    pass(IDLE_INPUT_MS * 2)
    expect(onInstall).not.toHaveBeenCalled()

    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    act(() => { window.dispatchEvent(new Event('blur')) })
    expect(onInstall).toHaveBeenCalledTimes(1)
  })

  it('never installs while an agent is working, and does as soon as it stops', () => {
    const { onInstall, rerender, initialProps } = setup({ agentBusy: true })
    pass(IDLE_INPUT_MS * 2)
    expect(onInstall).not.toHaveBeenCalled()

    rerender({ ...initialProps, agentBusy: false })
    expect(onInstall).toHaveBeenCalledTimes(1)
  })

  it('downloads a found update when idle', () => {
    const { onDownload, onInstall } = setup({ status: 'available' })
    pass(IDLE_INPUT_MS)
    expect(onDownload).toHaveBeenCalled()
    expect(onInstall).not.toHaveBeenCalled()
  })

  it('does nothing when the setting is off', () => {
    const { onInstall, onDownload } = setup({ enabled: false })
    pass(IDLE_INPUT_MS * 2)
    expect(onInstall).not.toHaveBeenCalled()
    expect(onDownload).not.toHaveBeenCalled()
  })
})
