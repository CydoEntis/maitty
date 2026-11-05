import { render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { WEB_SPINNERS } from '@/lib/spinner-clock'
import { WebSpinner } from './WebSpinner'

describe('web spinner', () => {
  it.each(WEB_SPINNERS)('draws %s as an inline SVG in the text colour inside its own layer', (kind) => {
    const { container } = render(<WebSpinner kind={kind} />)
    const span = container.firstElementChild

    expect(span).toHaveClass('mx-wspin')
    expect(span).toHaveAttribute('aria-hidden', 'true')
    expect(span?.querySelector('svg')).toHaveAttribute('fill', 'currentColor')
  })

  it('draws eight spokes at rising opacity', () => {
    const { container } = render(<WebSpinner kind="spokes" />)
    const opacities = Array.from(container.querySelectorAll('line')).map((line) => Number(line.getAttribute('opacity')))

    expect(opacities).toHaveLength(8)
    expect(opacities).toEqual([...opacities].sort((a, b) => a - b))
  })

  it('stands still when asked', () => {
    const { container } = render(<WebSpinner kind="circle" still />)

    expect(container.firstElementChild).toHaveClass('is-still')
  })
})
