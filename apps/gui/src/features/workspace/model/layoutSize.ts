import { UI_TEXT_SIZE_DEFAULT, type Density, type IconSize, type SettingsPreferences } from '@/features/settings/preferences'

/** How much each density stretches the sidebar's rows, the tabs and the panes' gaps. */
export const DENSITY_SCALE: Record<Density, number> = { compact: 0.8, comfortable: 1, spacious: 1.25 }

/** How much each icon size stretches the sidebar's and the rail's icons, tiles and the rail's width. */
export const ICON_SCALE: Record<IconSize, number> = { small: 0.8, medium: 1, large: 1.2 }

/**
 * The CSS variables the window's size rules read, so one setting resizes every rule that uses it.
 * The text variables are ratios to the default size, so a rule multiplies its own px size by one.
 */
/**
 * The widest the chat's column runs, for each choice. `auto` is the design's column on a small pane and grows with a
 * large one (to 62% of the pane, 1100px at most), so the chat is not a narrow strip on a big screen.
 */
const CHAT_WIDTH: Readonly<Record<SettingsPreferences['chatWidth'], string>> = {
  auto: 'max(760px, min(1100px, 62%))',
  narrow: '760px',
  wide: '1100px',
  full: '100%',
}
const PERCENT = 100

export function sizeVariables(preferences: Pick<SettingsPreferences, 'density' | 'sidebarTextSize' | 'tabTextSize' | 'iconSize' | 'chatWidth' | 'chatTextSize'>, sidebarWidth: number): Record<string, string> {
  return {
    '--pad': String(DENSITY_SCALE[preferences.density]),
    '--sidebar-w': `${String(sidebarWidth)}px`,
    '--sidebar-text': String(preferences.sidebarTextSize / UI_TEXT_SIZE_DEFAULT),
    '--icon-scale': String(ICON_SCALE[preferences.iconSize]),
    '--tab-text': String(preferences.tabTextSize / UI_TEXT_SIZE_DEFAULT),
    '--chat-width': CHAT_WIDTH[preferences.chatWidth],
    '--chat-zoom': String(preferences.chatTextSize / PERCENT),
  }
}
