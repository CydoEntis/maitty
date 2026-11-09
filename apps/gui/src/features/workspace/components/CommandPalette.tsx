import { useEffect, useMemo, useRef, useState } from 'react'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { ResultsList, ResultsOption } from '@/components/ui/results-list'
import { searchPalette, type PaletteItem } from '../model/palette'

/** ⌘K: jump to a session, project, action or theme by typing a few letters of its name. */
export function CommandPalette({ items, onClose }: { items: readonly PaletteItem[]; onClose: () => void }) {
  const [query, setQuery] = useState('')
  const [selected, setSelected] = useState(0)
  const results = useMemo(() => searchPalette(items, query), [items, query])
  const active = Math.min(selected, Math.max(0, results.length - 1))
  const list = useRef<HTMLDivElement>(null)

  useEffect(() => {
    list.current?.querySelector<HTMLElement>('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' })
  }, [active, results])

  const run = (item: PaletteItem | undefined) => {
    if (!item) return
    onClose()
    item.run()
  }
  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === 'Escape') {
      event.stopPropagation()
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      if (results.length > 0) setSelected((active + (event.key === 'ArrowDown' ? 1 : -1) + results.length) % results.length)
    } else if (event.key === 'Enter') {
      event.preventDefault()
      run(results[active])
    }
  }
  return (
    <Dialog open onOpenChange={(open) => { if (!open) onClose() }}>
      <DialogContent unstyled overlayClassName="mx-palette-scrim" aria-label="Command palette" aria-describedby={undefined} className="mx-palette" onKeyDown={onKeyDown} onOpenAutoFocus={(event) => { event.preventDefault() }}>
        <DialogTitle className="sr-only">Command palette</DialogTitle>
        <div className="mx-palette-input">
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
          <input
            autoFocus
            role="combobox"
            aria-expanded="true"
            aria-controls="mx-palette-list"
            aria-label="Search the palette"
            placeholder="Jump to a session, project, action or theme…"
            value={query}
            spellCheck={false}
            onChange={(event) => { setQuery(event.target.value); setSelected(0) }}
          />
          <kbd className="mx-keycap">esc</kbd>
        </div>
        <ResultsList ref={list} id="mx-palette-list" aria-label="Results" className="mx-palette-list">
          {results.map((item, index) => (
            <ResultsOption key={item.id} selected={index === active} className={`mx-palette-row${index === active ? ' is-selected' : ''}`} onMouseMove={() => { setSelected(index) }} onClick={() => { run(item) }}>
              <span className="mx-palette-label">{item.label}</span>
              {item.detail && <span className="mx-palette-detail">{item.detail}</span>}
              <span className="mx-spacer" />
              {item.shortcut && <span className="mx-palette-shortcut">{item.shortcut}</span>}
              <span className={`mx-palette-kind${item.badge ? ' is-badge' : ''}`}>{item.badge ?? item.kind}</span>
            </ResultsOption>
          ))}
          {results.length === 0 && <div className="mx-palette-empty">Nothing matches “{query}”.</div>}
        </ResultsList>
        <div className="mx-palette-foot"><span>↑ ↓ move</span><span>↵ open</span></div>
      </DialogContent>
    </Dialog>
  )
}
