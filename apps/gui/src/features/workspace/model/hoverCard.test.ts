import { describe, expect, it } from 'vitest'
import { HOVER_CARD_WIDTH_PX, placeHoverCard } from './hoverCard'

describe('where the hover card goes', () => {
  it('sits to the right of the row, level with it', () => {
    expect(placeHoverCard({ left: 20, right: 280, top: 200 }, 180, 1400, 900)).toEqual({ left: 288, top: 200 })
  })

  it('moves up so it never runs past the bottom of the window', () => {
    expect(placeHoverCard({ left: 20, right: 280, top: 800 }, 180, 1400, 900).top).toBe(712)
  })

  it('goes to the left of a sidebar that is on the right', () => {
    expect(placeHoverCard({ left: 1120, right: 1380, top: 200 }, 180, 1400, 900).left).toBe(1120 - 8 - HOVER_CARD_WIDTH_PX)
  })
})
