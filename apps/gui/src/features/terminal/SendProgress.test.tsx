import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { SendProgress } from './SendProgress'

describe('the progress of a file being sent', () => {
  it('is a named progress bar that says how far it has got', () => {
    render(<SendProgress file="report.pdf" percent={40} />)
    const bar = screen.getByRole('progressbar', { name: 'Sending report.pdf' })
    expect(bar).toHaveAttribute('aria-valuenow', '40')
    expect(bar).toHaveAttribute('aria-valuemin', '0')
    expect(bar).toHaveAttribute('aria-valuemax', '100')
    expect(screen.getByText('40%')).toBeInTheDocument()
  })

  it('has no value, and says no percentage, until the size is known', () => {
    render(<SendProgress file="report.pdf" percent={null} />)
    expect(screen.getByRole('progressbar')).not.toHaveAttribute('aria-valuenow')
    expect(screen.queryByText(/%/)).toBeNull()
  })

  it('never shows more than the whole', () => {
    render(<SendProgress file="a" percent={250} />)
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '100')
  })
})
