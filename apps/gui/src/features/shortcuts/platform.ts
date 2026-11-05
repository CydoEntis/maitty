import { createContext, useContext } from 'react'
import { detectPlatform, shortcutText, type Platform, type ShortcutAction } from '@/features/workspace/model/shortcuts'

/** The platform the window runs on, read once from the user agent. */
const PlatformContext = createContext<Platform>(detectPlatform(typeof navigator === 'undefined' ? '' : navigator.userAgent))

export const usePlatform = (): Platform => useContext(PlatformContext)

/** The platform's text for an action's key, such as `Ctrl+Shift+T` or `⌘T`; '' when the platform has none. */
export function useShortcutText(): (action: ShortcutAction) => string {
  const platform = usePlatform()
  return (action) => shortcutText(action, platform)
}

/** `Name (keys)` for a tooltip, or just the name when the platform has no key for the action. */
export function useTipWithKeys(): (name: string, action: ShortcutAction) => string {
  const text = useShortcutText()
  return (name, action) => {
    const keys = text(action)
    return keys === '' ? name : `${name} (${keys})`
  }
}
