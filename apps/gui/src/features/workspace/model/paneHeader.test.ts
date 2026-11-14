import { describe, expect, it } from 'vitest'
import { headerFit } from './paneHeader'

describe('pane header fit', () => {
  it('shows the view tabs in a pane wide enough', () => {
    expect(headerFit(400)).toEqual({ switcher: true })
  })

  it('gives up the view tabs in a narrow pane', () => {
    expect(headerFit(340)).toEqual({ switcher: false })
  })

  it('shows everything until the pane is measured', () => {
    expect(headerFit(null)).toEqual({ switcher: true })
  })
})
