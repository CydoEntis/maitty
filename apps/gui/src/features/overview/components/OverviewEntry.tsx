import { cn } from '@/lib/utils'
import { useTipWithKeys } from '@/features/shortcuts/platform'
import { Icon } from '@/features/workspace/components/glyphs'

export type OverviewEntryProps = {
  open: boolean
  /** Sessions that need you; the amber badge is hidden at 0. */
  needs: number
  onToggle: () => void
}

/** Icon sizes in px: the docked row's and the rail's. */
const ROW_ICON = 15
const RAIL_ICON = 17

const badgeLabel = (needs: number) => `${String(needs)} ${needs === 1 ? 'needs' : 'need'} you`

/** The sidebar's pinned Overview row, and the 40 by 36 rail icon, with the needs-you count as an amber badge. */
export function OverviewEntry({ open, needs, onToggle, rail = false }: OverviewEntryProps & { rail?: boolean }) {
  const tip = useTipWithKeys()
  const badge = needs > 0 && (
    <span
      aria-hidden="true"
      className={cn('flex h-[18px] min-w-[18px] items-center justify-center rounded-full bg-[var(--warn)] px-1.5 text-[11px] font-semibold text-[var(--on-warn)] tabular-nums', rail && 'absolute -top-0.5 -right-0.5 h-4 min-w-4 px-1 text-[10px]')}
    >
      {needs}
    </span>
  )
  return (
    <button
      type="button"
      aria-label={needs > 0 ? `Overview, ${badgeLabel(needs)}` : 'Overview'}
      aria-pressed={open}
      title={tip(needs > 0 ? `Overview · ${badgeLabel(needs)}` : 'Overview', 'overview')}
      className={cn(
        'relative flex items-center rounded-lg text-fg2 transition-colors hover:bg-hover hover:text-fg',
        rail ? 'h-9 w-10 justify-center' : 'mx-2 mb-1 h-8 gap-2.5 px-2.5 text-[13px] font-medium',
        open && 'bg-sidebar-accent text-fg',
      )}
      onClick={onToggle}
    >
      <Icon name="grid" size={rail ? RAIL_ICON : ROW_ICON} />
      {!rail && <span className="flex-1 text-left">Overview</span>}
      {badge}
    </button>
  )
}
