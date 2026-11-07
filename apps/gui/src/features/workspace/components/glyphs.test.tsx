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
  it('gives idle, done, needs you, exited and unknown each their own shape', () => {
    const shapes = (['idle', 'done', 'needs', 'error', 'unknown'] as const).map((status) => {
      const { container } = render(<StatusBadge status={status} rail />)
      return container.firstElementChild?.className ?? ''
    })
    expect(shapes).toEqual(['mx-badge is-idle', 'mx-badge is-done', 'mx-badge is-needs', 'mx-badge is-error', 'mx-badge is-unknown'])
  })

  it('draws idle as two z\'s and exited as a bang, with no word in the mark itself', () => {
    const idle = render(<StatusBadge status="idle" />)
    expect(idle.container.querySelector('.mx-zz')?.textContent).toBe('zz')
    const exited = render(<StatusBadge status="error" />)
    expect(exited.container.querySelector('svg')).not.toBeNull()
  })

  it('still draws unknown in a row, not only in the rail', () => {
    const { container } = render(<StatusBadge status="unknown" />)
    expect(container.querySelector('.is-unknown')).not.toBeNull()
  })
})
