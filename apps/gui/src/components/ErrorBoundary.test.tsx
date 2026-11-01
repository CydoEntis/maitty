import { render, screen } from '@testing-library/react'
import type { ReactNode } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import ErrorBoundary from './ErrorBoundary'

describe('ErrorBoundary', () => {
  afterEach(() => vi.restoreAllMocks())

  it('shows a reload action when a child crashes', () => {
    vi.spyOn(console, 'error').mockImplementation(() => undefined)
    function BrokenWidget(): ReactNode {
      throw new Error('private internal details')
    }

    render(<ErrorBoundary><BrokenWidget /></ErrorBoundary>)

    expect(screen.getByRole('alert')).toHaveTextContent('Maitty hit a problem')
    expect(screen.getByRole('button', { name: 'Reload app' })).toBeInTheDocument()
    expect(screen.queryByText('private internal details')).not.toBeInTheDocument()
  })
})
