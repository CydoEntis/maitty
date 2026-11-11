import { beforeEach, describe, expect, it, vi } from 'vitest'

const custom = vi.hoisted(() => vi.fn(() => 'shown'))
vi.mock('sonner', () => ({ toast: { custom, dismiss: vi.fn() } }))

import { toast } from './toast'

describe('toast.error', () => {
  beforeEach(() => { custom.mockClear() })

  it('stays quiet when the only news is that the window has no line to the service', () => {
    toast.error('The terminal is not connected to the daemon')

    expect(custom).not.toHaveBeenCalled()
  })

  it('shows every other failure', () => {
    toast.error('Could not copy the selection')

    expect(custom).toHaveBeenCalledTimes(1)
  })
})
