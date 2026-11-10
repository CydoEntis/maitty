import { describe, expect, it } from 'vitest'
import { DENSITY_SCALE, ICON_SCALE, sizeVariables } from './layoutSize'

describe('window size variables', () => {
  it('turns the density, text sizes and sidebar width into CSS variables', () => {
    expect(sizeVariables({ density: 'spacious', sidebarTextSize: 27, tabTextSize: 13.5, iconSize: 'small' }, 300)).toEqual({
      '--pad': String(DENSITY_SCALE.spacious),
      '--sidebar-w': '300px',
      '--sidebar-text': '2',
      '--tab-text': '1',
      '--icon-scale': String(ICON_SCALE.small),
    })
  })

  it('makes compact tighter and spacious roomier than comfortable', () => {
    expect(DENSITY_SCALE.compact).toBeLessThan(DENSITY_SCALE.comfortable)
    expect(DENSITY_SCALE.spacious).toBeGreaterThan(DENSITY_SCALE.comfortable)
  })

  it('makes small icons smaller and large icons larger than medium', () => {
    expect(ICON_SCALE.small).toBeLessThan(ICON_SCALE.medium)
    expect(ICON_SCALE.large).toBeGreaterThan(ICON_SCALE.medium)
  })
})
