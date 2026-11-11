import { ChoiceItem, ChoiceList } from '@/components/ui/choice-list'
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
  /** Shown under the list and above the footer, outside the filter (a meter that comes after the choice above it). */
  below?: ReactNode
  /** A line the search can turn into a row of its own (a pasted path); null when it cannot. */
  fromQuery?: (query: string) => ChipItem | null
  width?: number
  /** Which way the list opens from the chip: up from the message box, or down from the top of a page. */
  side?: 'top' | 'bottom'
}

/** One fact of the context row over the message box, as a small chip that opens a searchable list above it. */
export function ChipMenu({ label, summary, title, icon, search, sections, footer, extra, below, fromQuery, width = 340, side = 'top' }: Props) {
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
      <PopupContent side={side} align="start" aria-label={title} className="mx-chip-menu" style={{ width: `min(${String(width)}px, calc(100vw - 32px))` }}>
        {search !== undefined && (
          <label className="mx-chip-search">
            <Icon name="search" size={13} />
            <input autoFocus aria-label={search} placeholder={search} value={query} onChange={(event) => { setQuery(event.target.value) }}
              onKeyDown={(event) => { if (event.key === 'Enter' && made) { event.preventDefault(); pick(made) } }} />
            <span className="mx-chip-count">{total}</span>
          </label>
        )}
        {extra}
        <ChoiceList className="mx-chip-list" aria-label={title} current={kept.flatMap((section) => section.items).find((item) => item.checked)?.key ?? null}>
          {made && <ChipRow item={made} onPick={pick} />}
          {kept.map((section) => (
            <div key={section.heading} role="group" aria-label={section.heading}>
              <h3 className="mx-chip-heading"><span>{section.heading}</span><span>{section.items.length}</span></h3>
              {section.items.map((item) => <ChipRow key={item.key} item={item} onPick={pick} />)}
            </div>
          ))}
          {kept.length === 0 && !made && <p className="mx-chip-none">Nothing matches</p>}
        </ChoiceList>
        {below}
        {footer !== undefined && <div className="mx-chip-footer">{footer}</div>}
      </PopupContent>
    </Popover.Root>
  )
}

function ChipRow({ item, onPick }: { item: ChipItem; onPick: (item: ChipItem) => void }) {
  return (
    <ChoiceItem value={item.key} disabled={item.disabled} className={`mx-chip-row${item.checked ? ' is-checked' : ''}`} onPick={() => { onPick(item) }}>
      {item.icon}
      <span className="mx-chip-row-text">
        <span className="mx-chip-row-label">{item.label}{item.badge !== undefined && <span className="mx-chip-badge">{item.badge}</span>}</span>
        {item.sub !== undefined && <span className="mx-chip-row-sub">{item.sub}</span>}
      </span>
      {item.right}
      {item.checked && <Icon name="check" size={12} stroke="var(--accent)" strokeWidth={2.4} />}
    </ChoiceItem>
  )
}
