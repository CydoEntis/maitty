import { describe, expect, it } from 'vitest'
import { galleryHash, galleryParams, isGalleryHash } from './galleryParams'

describe('the gallery hash', () => {
  it('opens on #gallery alone or with a query, and on nothing else', () => {
    expect([isGalleryHash('#gallery'), isGalleryHash('#gallery?theme=Paper'), isGalleryHash('#galleryx'), isGalleryHash('')]).toEqual([true, true, false, false])
  })

  it('reads the theme and corners it links to, falling back to Night and square', () => {
    expect(galleryParams(galleryHash('Paper', 'rounded'))).toEqual({ theme: 'Paper', corners: 'rounded', open: null })
    expect(galleryParams('#gallery?corners=wavy')).toEqual({ theme: 'Maitty Night', corners: 'square', open: null })
    expect(galleryParams('#gallery?theme=Paper&open=run').open).toBe('run')
  })
})
