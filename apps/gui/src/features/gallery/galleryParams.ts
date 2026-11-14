import { DEFAULT_CORNERS, isCorners, type Corners } from '@/features/workspace/theme/corners'

/** The hash that opens the gallery: `#gallery`, optionally `#gallery?theme=Paper&corners=rounded`. */
export const GALLERY_HASH = '#gallery'

export function isGalleryHash(hash: string): boolean {
  return hash === GALLERY_HASH || hash.startsWith(`${GALLERY_HASH}?`)
}

/** The theme name and corners the gallery hash asks for; an unknown theme is left to `findTheme`'s fallback. */
export function galleryParams(hash: string): { theme: string; corners: Corners } {
  const query = new URLSearchParams(hash.slice(hash.indexOf('?') + 1))
  const corners = query.get('corners')
  return { theme: query.get('theme') ?? 'Maitty Night', corners: isCorners(corners) ? corners : DEFAULT_CORNERS }
}

export function galleryHash(theme: string, corners: Corners): string {
  return `${GALLERY_HASH}?${new URLSearchParams({ theme, corners }).toString()}`
}
