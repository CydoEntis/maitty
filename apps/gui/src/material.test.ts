/// <reference types="node" />
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

// Vitest hands every stylesheet to a test as an empty string (`?raw` included), so the glob only names the files and
// their text is read from disk.
const SHEETS: Record<string, string> = Object.fromEntries(
  Object.keys(import.meta.glob('./**/*.css')).map((path) => [path, readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8')]),
)

/** The stylesheets that are not the material itself or the theme tokens it reads. */
const surfaces = Object.entries(SHEETS).filter(([path]) => !path.endsWith('material.css') && !path.endsWith('index.css') && !path.endsWith('/tokens.css'))

describe('the window material', () => {
  it('reads the stylesheets it checks', () => {
    expect(Object.values(SHEETS).every((css) => css.length > 0)).toBe(true)
    expect(Object.keys(SHEETS)).toContain('./material.css')
  })

  // A surface reads the role (`--m-well`, `--m-raise`, ...), not the theme colour, so the role can change for every
  // surface in one place.
  it('is the only place that reads the solid surface colours', () => {
    const offenders = surfaces.filter(([, css]) => /var\(--(well|raise|sel|hover|pane)\)/.test(css)).map(([path]) => path)
    expect(offenders).toEqual([])
  })

  it('does not tint with a fixed white, which disappears on a light theme', () => {
    const offenders = surfaces.filter(([, css]) => /rgba\(255,\s*255,\s*255,\s*\.\d+\)/.test(css)).map(([path]) => path)
    expect(offenders).toEqual([])
  })

  // The glass finish is gone: its tokens are not declared any more, so a rule that read one would silently draw nothing.
  it('reads no token of the removed glass finish', () => {
    const removed = /var\(--(m-sheen|m-edge|m-shadow|m-blur|m-line|m-panel|m-float|m-field|gk|blur-pop)\b/
    const offenders = Object.entries(SHEETS).filter(([, css]) => removed.test(css)).map(([path]) => path)
    expect(offenders).toEqual([])
  })

  it('has one finish, so no rule asks which finish is on and nothing blurs what is behind it', () => {
    const offenders = Object.entries(SHEETS).filter(([, css]) => /data-finish|backdrop-filter/.test(css)).map(([path]) => path)
    expect(offenders).toEqual([])
  })
})
