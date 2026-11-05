/**
 * Files dropped on a chat composer. The desktop shell takes file drops for itself (see
 * `useFileDrop`), so the composer cannot listen for them; it registers its element here and the
 * window's one drop handler asks whether the drop landed on a composer before it routes the files to
 * a pane.
 */

type Target = { readonly element: Element; readonly onFiles: (paths: readonly string[]) => void }

const targets = new Set<Target>()

/** Takes drops on `element` for `onFiles`. Returns the function that stops. */
export function registerComposerDrop(element: Element, onFiles: (paths: readonly string[]) => void): () => void {
  const target: Target = { element, onFiles }
  targets.add(target)
  return () => { targets.delete(target) }
}

/** Gives the files to the composer that holds `under` (the element at the drop point); false when none does. */
export function dropOnComposer(paths: readonly string[], under: Element | null): boolean {
  if (under === null) return false
  for (const target of targets) {
    if (!target.element.contains(under)) continue
    target.onFiles(paths)
    return true
  }
  return false
}
