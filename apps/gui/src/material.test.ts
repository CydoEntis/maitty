import { describe, expect, it } from 'vitest'

const SHEETS = import.meta.glob<string>('./**/*.css', { query: '?raw', import: 'default', eager: true })

/** The stylesheets that are not the material itself or the theme tokens it reads. */
const surfaces = Object.entries(SHEETS).filter(([path]) => !path.endsWith('material.css') && !path.endsWith('index.css') && !path.endsWith('/tokens.css'))

describe('the window material', () => {
  // The theme's solid colours are what Flat shows. Glass swaps them for tints, so a stylesheet reads the
  // role (`--m-well`, `--m-raise`, ...) and never the theme colour, or that surface stays solid in Glass.
  it('is the only place that reads the solid surface colours', () => {
    const offenders = surfaces.filter(([, css]) => /var\(--(well|raise|sel|hover|pane)\)/.test(css)).map(([path]) => path)
    expect(offenders).toEqual([])
  })

  it('does not tint with a fixed white, which disappears on a light theme', () => {
    const offenders = surfaces.filter(([, css]) => /rgba\(255,\s*255,\s*255,\s*\.\d+\)/.test(css)).map(([path]) => path)
    expect(offenders).toEqual([])
  })
})
