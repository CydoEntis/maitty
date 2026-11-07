import { describe, expect, it } from 'vitest'
import { headerFit } from './paneHeader'

describe('the pane header as the pane narrows', () => {
  it('shows every control on a wide pane', () => {
    expect(headerFit(900)).toEqual({ toolLabels: true, splits: true, tools: true, switcher: true })
  })

  it('drops the Files and Changes labels first', () => {
    expect(headerFit(640)).toEqual({ toolLabels: false, splits: true, tools: true, switcher: true })
  })

  it('then the split buttons', () => {
    expect(headerFit(560)).toEqual({ toolLabels: false, splits: false, tools: true, switcher: true })
  })

  it('then Files and Changes', () => {
    expect(headerFit(480)).toEqual({ toolLabels: false, splits: false, tools: false, switcher: true })
  })

  it('and last the terminal | chat switch', () => {
    expect(headerFit(400)).toEqual({ toolLabels: false, splits: false, tools: false, switcher: false })
  })

  it('shows everything before the pane is measured', () => {
    expect(headerFit(null).switcher).toBe(true)
  })
})
