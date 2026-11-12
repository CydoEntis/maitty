import { render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { StatusBadge } from './glyphs'

describe('the working badge', () => {
  it('turns a ring inside its square, not text, whatever spinner Settings picked', () => {
    const { container } = render(<StatusBadge status="working" />)
    expect(container.querySelector('.mx-badge.is-working .mx-wspin[data-kind="circle"] svg')).not.toBeNull()
    expect(container.querySelector('.mx-badge.is-working .mx-spin')).toBeNull()
  })
})
