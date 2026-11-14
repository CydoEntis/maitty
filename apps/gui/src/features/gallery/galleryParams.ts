import { DEFAULT_CORNERS, isCorners, type Corners } from '@/features/workspace/theme/corners'

/** The hash that opens the gallery: `#gallery`, optionally `#gallery?theme=Paper&corners=rounded`. */
export const GALLERY_HASH = '#gallery'

export function isGalleryHash(hash: string): boolean {
  return hash === GALLERY_HASH || hash.startsWith(`${GALLERY_HASH}?`)
}

/**
 * The theme name and corners the gallery hash asks for; an unknown theme is left to `findTheme`'s fallback. `open`
 * names a sample drawn alone: the Work board (`open=work`), or with its run drawer open (`open=run`), since a modal dialog takes the whole page.
 */
export function galleryParams(hash: string): { theme: string; corners: Corners; open: string | null } {
  const query = new URLSearchParams(hash.slice(hash.indexOf('?') + 1))
  const corners = query.get('corners')
  return { theme: query.get('theme') ?? 'Maitty Night', corners: isCorners(corners) ? corners : DEFAULT_CORNERS, open: query.get('open') }
}

export function galleryHash(theme: string, corners: Corners): string {
  return `${GALLERY_HASH}?${new URLSearchParams({ theme, corners }).toString()}`
}
