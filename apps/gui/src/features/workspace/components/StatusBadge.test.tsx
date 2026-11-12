import { act, render } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { setSpinnerStyle } from '@/lib/spinner-clock'
import { StatusBadge } from './glyphs'

afterEach(() => { act(() => { setSpinnerStyle('braille') }) })

describe('the working badge', () => {
  it('shows the spinner Settings picked: its text frames, or its web spinner', () => {
    const { container } = render(<StatusBadge status="working" />)
    expect(container.querySelector('.mx-badge.is-working .mx-wspin')).toBeNull()

    act(() => { setSpinnerStyle('circle') })
    expect(container.querySelector('.mx-badge.is-working .mx-wspin[data-kind="circle"]')).not.toBeNull()
  })
})
