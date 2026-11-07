import type { CSSProperties, ReactNode } from 'react'
import { Icon } from './glyphs'

/**
 * A section that folds away. The height moves as `grid-template-rows` 1fr to 0fr (a measured
 * exception to the transform-and-opacity rule, docs/PERFORMANCE.md) and the content fades; folded
 * content is `inert` and `aria-hidden`, so the keyboard and a screen reader skip it.
 */
export function Collapse({ open, children }: { open: boolean; children: ReactNode }) {
  return (
    <div className="mx-collapse-grid" data-open={open}>
      <div className="mx-collapse-inner" aria-hidden={open ? undefined : true} inert={!open}>{children}</div>
    </div>
  )
}

type HeadProps = {
  label: string
  open: boolean
  onToggle: () => void
  /** The count shown after the label. */
  count?: number
  /** A mark before the label (a machine's laptop or server). */
  icon?: ReactNode
  /** Controls at the right edge; they sit beside the toggle, not inside it. */
  children?: ReactNode
  className?: string
  style?: CSSProperties | undefined
}

/** A section heading: the caret turns a quarter when it folds. */
export function SectionHead({ label, open, onToggle, count, icon, children, className = '', style }: HeadProps) {
  return (
    <div className={`mx-section-head ${className}`} style={style}>
      <button type="button" className="mx-section-toggle" aria-expanded={open} onClick={onToggle}>
        <span className={`mx-caret-wrap${open ? ' is-open' : ''}`}><Icon name="caret" /></span>
        {icon && <span className="mx-section-icon" aria-hidden="true">{icon}</span>}
        <span className="mx-section-label">{label}</span>
        {count !== undefined && <span className="mx-section-count">{count}</span>}
      </button>
      {children && <span className="mx-section-actions">{children}</span>}
    </div>
  )
}
