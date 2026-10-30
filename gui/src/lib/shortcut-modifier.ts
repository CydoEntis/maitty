export function shortcutModifier(userAgent: string): '⌘' | 'Ctrl' {
  return /Macintosh|Mac OS X/i.test(userAgent) ? '⌘' : 'Ctrl'
}
