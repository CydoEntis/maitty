import { useCallback, useState, type CSSProperties, type ReactNode } from 'react'
import { ChevronDownIcon } from 'lucide-react'
import { DropdownMenu, DropdownMenuContent, DropdownMenuLabel, DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'

/** One choice of a picker: its mark, its name and a line under it. */
export type PickerItem = {
  value: string
  label: string
  sub?: string
  /** A small picture of the choice: the font's `Aa`, the style's colours, the spinner turning. */
  tile: ReactNode
  /** Sets the name in its own typeface, for a font. */
  labelStyle?: CSSProperties
}

/** Choices under an optional heading. */
export type PickerGroup = { label?: string; items: readonly PickerItem[] }

const ITEM_CLASS = 'h-auto min-h-10 gap-2.5 py-1.5 pr-8 pl-1.5 text-[12.5px] text-fg focus:bg-hover focus:text-fg [&_[data-slot=dropdown-menu-radio-item-indicator]]:text-brand'
const GROUP_LABEL_CLASS = 'px-1.5 pt-1.5 pb-1 text-[10.5px] font-semibold tracking-wider text-dim uppercase'

/**
 * The one dropdown of Settings: a button showing the choice (its mark, name and line, as the theme pickers
 * show theirs) and a list. It is Radix's menu, so Tab reaches it, Enter, Space and the arrows open it, the
 * arrows, Home, End and typing a name move through it, Enter picks, and Esc closes it without closing
 * Settings. The list mounts in the app root, where the theme's colours are set.
 */
export function PickerSelect({ label, value, groups, onPick, width }: { label: string; value: string; groups: readonly PickerGroup[]; onPick: (value: string) => void; width?: number }) {
  const [container, setContainer] = useState<HTMLElement | null>(null)
  const findRoot = useCallback((button: HTMLButtonElement | null) => { setContainer(button?.closest<HTMLElement>('.mx-app') ?? null) }, [])
  const current = groups.flatMap((group) => group.items).find((item) => item.value === value)
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button ref={findRoot} type="button" aria-label={`${label}: ${current?.label ?? value}`} className="mx-picker-button" style={width === undefined ? undefined : { width }}>
          {current?.tile}
          <span className="mx-picker-text">
            <span className="mx-theme-name" style={current?.labelStyle}>{current?.label ?? value}</span>
            {current?.sub !== undefined && <span className="mx-theme-sub">{current.sub}</span>}
          </span>
          <ChevronDownIcon className="mx-picker-chevron size-4" aria-hidden="true" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        container={container}
        align="start"
        aria-label={label}
        className="max-h-[min(340px,var(--radix-dropdown-menu-content-available-height))] w-[300px] text-fg"
        onEscapeKeyDown={(event) => { event.stopPropagation() }}
      >
        <DropdownMenuRadioGroup value={value} onValueChange={onPick}>
          {groups.map((group, index) => (
            <div key={group.label ?? String(index)}>
              {index > 0 && <DropdownMenuSeparator className="bg-pane-line" />}
              {group.label !== undefined && <DropdownMenuLabel className={GROUP_LABEL_CLASS}>{group.label}</DropdownMenuLabel>}
              {group.items.map((item) => (
                <DropdownMenuRadioItem key={item.value} value={item.value} className={ITEM_CLASS}>
                  {item.tile}
                  <span className="mx-picker-text">
                    <span className="mx-theme-name" style={item.labelStyle}>{item.label}</span>
                    {item.sub !== undefined && <span className="mx-theme-sub">{item.sub}</span>}
                  </span>
                </DropdownMenuRadioItem>
              ))}
            </div>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
