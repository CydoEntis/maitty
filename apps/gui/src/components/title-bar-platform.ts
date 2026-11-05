import { isMacPlatform } from '@/features/workspace/model/shortcuts'

export type TitleBarPlatform = 'macos' | 'windows' | 'linux'

/** The platform decides only where the window buttons sit and how they look; nothing else in the bar differs. */
export function titleBarPlatform(userAgent: string): TitleBarPlatform {
  if (isMacPlatform(userAgent)) return 'macos'
  return /Windows/i.test(userAgent) ? 'windows' : 'linux'
}
