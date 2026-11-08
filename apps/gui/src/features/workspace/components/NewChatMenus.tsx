import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { useState, type ReactNode } from 'react'
import { Popover } from 'radix-ui'
import { PopupContent } from '@/components/ui/popup'
import { Icon } from './glyphs'

/** One row of a chip's list: a name, what is under it, and what sits at its right. */
export type ChipItem = { key: string; label: string; sub?: string; icon?: ReactNode; badge?: string; right?: ReactNode; checked: boolean; disabled?: boolean; onPick: () => void }
export type ChipSection = { heading: string; items: readonly ChipItem[] }

type Props = {
  /** What the chip says (the choice made). */
  label: ReactNode
  /** The chip's choice in words, for a screen reader, when the label is more than text. */
  summary?: string
  title: string
  icon?: ReactNode
  /** Given: a search box over the rows, with this placeholder and the number of rows beside it. */
  search?: string
  sections: readonly ChipSection[]
  /** Under the list: actions and a short note. */
  footer?: ReactNode
  /** Shown above the sections, outside the filter (a meter, a switch). */
  extra?: ReactNode
  /** A line the search can turn into a row of its own (a pasted path); null when it cannot. */
  fromQuery?: (query: string) => ChipItem | null
  width?: number
}

/** One fact of the context row over the message box, as a small chip that opens a searchable list above it. */
export function ChipMenu({ label, summary, title, icon, search, sections, footer, extra, fromQuery, width = 340 }: Props) {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const needle = query.trim().toLowerCase()
  const kept = sections
    .map((section) => ({ ...section, items: section.items.filter((item) => needle === '' || `${item.label} ${item.sub ?? ''}`.toLowerCase().includes(needle)) }))
    .filter((section) => section.items.length > 0)
  const made = needle === '' ? null : fromQuery?.(query.trim()) ?? null
  const total = sections.reduce((sum, section) => sum + section.items.length, 0)
  const pick = (item: ChipItem) => { setOpen(false); setQuery(''); item.onPick() }
  return (
    <Popover.Root open={open} onOpenChange={(next) => { setOpen(next); if (!next) setQuery('') }}>
      <Popover.Trigger asChild>
        <button type="button" className="mx-chip" title={title} aria-label={`${title}: ${summary ?? (typeof label === 'string' ? label : '')}`}>{icon}{label}<Icon name="chevronDown" /></button>
      </Popover.Trigger>
      <PopupContent side="top" align="start" aria-label={title} className="mx-chip-menu" style={{ width: `min(${String(width)}px, calc(100vw - 32px))` }}>
        {search !== undefined && (
          <label className="mx-chip-search">
            <Icon name="search" size={13} />
            <input autoFocus aria-label={search} placeholder={search} value={query} onChange={(event) => { setQuery(event.target.value) }}
              onKeyDown={(event) => { if (event.key === 'Enter' && made) { event.preventDefault(); pick(made) } }} />
            <span className="mx-chip-count">{total}</span>
          </label>
        )}
        {extra}
        <div className="mx-chip-list" role="menu">
          {made && <ChipRow item={made} onPick={pick} />}
          {kept.map((section) => (
            <div key={section.heading} role="group" aria-label={section.heading}>
              <h3 className="mx-chip-heading"><span>{section.heading}</span><span>{section.items.length}</span></h3>
              {section.items.map((item) => <ChipRow key={item.key} item={item} onPick={pick} />)}
            </div>
          ))}
          {kept.length === 0 && !made && <p className="mx-chip-none">Nothing matches</p>}
        </div>
        {footer !== undefined && <div className="mx-chip-footer">{footer}</div>}
      </PopupContent>
    </Popover.Root>
  )
}

function ChipRow({ item, onPick }: { item: ChipItem; onPick: (item: ChipItem) => void }) {
  return (
    <button type="button" role="menuitemradio" aria-checked={item.checked} disabled={item.disabled} className={`mx-chip-row${item.checked ? ' is-checked' : ''}`} onClick={() => { onPick(item) }}>
      {item.icon}
      <span className="mx-chip-row-text">
        <span className="mx-chip-row-label">{item.label}{item.badge !== undefined && <span className="mx-chip-badge">{item.badge}</span>}</span>
        {item.sub !== undefined && <span className="mx-chip-row-sub">{item.sub}</span>}
      </span>
      {item.right}
      {item.checked && <Icon name="check" size={12} stroke="var(--accent)" strokeWidth={2.4} />}
    </button>
  )
}

/** The reasoning levels as a row of segments under the model list: Auto sends nothing. */
export function ReasoningMeter<T extends string>({ levels, value, labels, onPick }: { levels: readonly T[]; value: T; labels: Readonly<Record<T, string>>; onPick: (level: T) => void }) {
  return (
    <RadioGroup className="mx-chip-meter" aria-label="Reasoning" orientation="horizontal" value={value} onValueChange={(next) => { const picked = levels.find((level) => level === next); if (picked !== undefined) onPick(picked) }}>
      <p className="mx-chip-meter-head"><strong>Reasoning</strong><span>{labels[value]}</span></p>
      <div className="mx-chip-meter-bars">
        {levels.map((level) => (
          <RadioGroupItem key={level} value={level} className={`mx-chip-meter-step${level === value ? ' is-on' : ''}`}>
            <span className="mx-chip-meter-bar" aria-hidden="true" />
            <span>{labels[level]}</span>
          </RadioGroupItem>
        ))}
      </div>
    </RadioGroup>
  )
}

/** Two or three choices as a row of tabs at the top of a list (this checkout or a new worktree). */
export function SegmentTabs<T extends string>({ value, options, onPick }: { value: T; options: readonly { value: T; label: string }[]; onPick: (value: T) => void }) {
  return (
    <div className="mx-chip-tabs" role="tablist">
      {options.map((option) => (
        <button key={option.value} type="button" role="tab" aria-selected={option.value === value} className={`mx-chip-tab${option.value === value ? ' is-on' : ''}`} onClick={() => { onPick(option.value) }}>{option.label}</button>
      ))}
    </div>
  )
}
