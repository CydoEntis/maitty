import { describe, expect, it, vi } from 'vitest'
import { announceOutsideSend, onOutsideSend } from './outsideSends'

describe('a message sent from outside the message box', () => {
  it('reaches the chat of its own conversation, until that chat stops listening', () => {
    const mine = vi.fn()
    const other = vi.fn()
    const stop = onOutsideSend(7, mine)
    const stopOther = onOutsideSend(8, other)
    announceOutsideSend(7, 'rename this')
    expect(mine).toHaveBeenCalledWith('rename this')
    expect(other).not.toHaveBeenCalled()
    stop()
    announceOutsideSend(7, 'again')
    expect(mine).toHaveBeenCalledTimes(1)
    stopOther()
  })
})
