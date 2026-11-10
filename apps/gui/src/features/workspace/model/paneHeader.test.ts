import { describe, expect, it } from 'vitest'
import { headerFit } from './paneHeader'

describe('pane header fit', () => {
  it('shows every control in a wide pane', () => {
    expect(headerFit(900)).toEqual({ split: true, switcher: true })
  })

  it('gives up Split right first as the pane narrows', () => {
    expect(headerFit(400)).toEqual({ split: false, switcher: true })
  })

  it('gives up the terminal | chat switch last', () => {
    expect(headerFit(340)).toEqual({ split: false, switcher: false })
  })

  it('shows everything until the pane is measured', () => {
    expect(headerFit(null)).toEqual({ split: true, switcher: true })
  })
})
