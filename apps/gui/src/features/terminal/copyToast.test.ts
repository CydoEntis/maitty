import { describe, expect, it } from 'vitest'
import { LOCAL_MACHINE } from '@/api/machines'
import { copySaid } from './copyToast'

describe('what a copy made by a program in a pane says', () => {
  it('is a plain "Copied" for this computer', () => {
    expect(copySaid(LOCAL_MACHINE, 'A machine', 12).done).toBe('Copied')
  })

  it('names another machine and how much it put on the clipboard', () => {
    expect(copySaid(1, 'box', 12)).toEqual({ done: 'box copied 12 characters to your clipboard', failed: 'Could not copy what box sent' })
  })
})
