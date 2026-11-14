import { render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { iconSaysProcess } from '../model/session'
import { AppIcon, StatusBadge } from './glyphs'

describe('app icons', () => {
  it.each(['claude', 'codex', 'deepseek', 'gemini', 'opencode', 'cargo', 'rustc', 'shell'])(
    '%s has an icon of its own, so its name is not also written beside it',
    (program) => {
      const { container } = render(<AppIcon process={program} size={24} />)
      expect(container.querySelector('.is-tinted')).toBeNull()
      expect(iconSaysProcess(program)).toBe(true)
    },
  )

  it('keeps the name of a program that has no icon, since a tinted letter does not say what it is', () => {
    const { container } = render(<AppIcon process="vim" size={24} />)
    expect(container.querySelector('.is-tinted')).not.toBeNull()
    expect(iconSaysProcess('vim')).toBe(false)
  })
})

describe('status marks', () => {
  it('gives working, done, needs you, exited and unknown each their own shape', () => {
    const shapes = (['working', 'done', 'needs', 'error', 'unknown'] as const).map((status) => {
      const { container } = render(<StatusBadge status={status} />)
      return container.firstElementChild?.className ?? ''
    })
    expect(shapes).toEqual(['mx-badge is-working', 'mx-badge is-done', 'mx-badge is-needs', 'mx-badge is-error', 'mx-badge is-unknown'])
  })

  it('marks needs you with !, stopped with × and done with ✓, hidden from screen readers since the row says the status', () => {
    const marks = (['needs', 'error', 'done'] as const).map((status) => {
      const { container } = render(<StatusBadge status={status} />)
      expect(container.firstElementChild?.getAttribute('aria-hidden')).toBe('true')
      return container.textContent
    })
    expect(marks).toEqual(['!', '×', '✓'])
  })

  it('draws no mark for an idle session: its tile dims instead', () => {
    const { container } = render(<StatusBadge status="idle" />)
    expect(container.firstElementChild).toBeNull()
  })

  it('still draws unknown in a row, not only in the rail', () => {
    const { container } = render(<StatusBadge status="unknown" />)
    expect(container.querySelector('.is-unknown')).not.toBeNull()
  })
})
