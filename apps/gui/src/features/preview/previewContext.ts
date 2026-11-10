import { createContext, useContext } from 'react'
import type { usePreviewState } from './usePreviewState'

/** Everything the preview app's screens share: the conversations, the drafts, what is open, and what each action does. */
type Preview = ReturnType<typeof usePreviewState>

export const PreviewContext = createContext<Preview | null>(null)

/** The preview app's shared state, for a screen rendered inside `PreviewApp`. */
export function usePreview(): Preview {
  const preview = useContext(PreviewContext)
  if (preview === null) throw new Error('usePreview is only for the screens inside PreviewApp')
  return preview
}
