import type { Density, IconSize, SettingsPreferences } from '@/features/settings/preferences'

/** How much each density stretches the sidebar's rows, the tabs and the panes' gaps. */
export const DENSITY_SCALE: Record<Density, number> = { compact: 0.8, comfortable: 1, spacious: 1.25 }

/** How much each icon size stretches the sidebar's and the rail's icons, tiles and the rail's width. */
export const ICON_SCALE: Record<IconSize, number> = { small: 0.8, medium: 1, large: 1.2 }

/** The CSS variables the window's size rules read, so one setting resizes every rule that uses it. */
export function sizeVariables(preferences: Pick<SettingsPreferences, 'density' | 'sidebarTextSize' | 'tabTextSize' | 'iconSize'>, sidebarWidth: number): Record<string, string> {
  return {
    '--pad': String(DENSITY_SCALE[preferences.density]),
    '--sidebar-w': `${String(sidebarWidth)}px`,
    '--sidebar-fs': `${String(preferences.sidebarTextSize)}px`,
    '--icon-scale': String(ICON_SCALE[preferences.iconSize]),
    '--tab-fs': `${String(preferences.tabTextSize)}px`,
  }
}
