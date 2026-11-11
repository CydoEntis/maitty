import { render } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { isDesktop } from '@/api/daemon'
import { installUndoRedo } from '@/features/editing/undoRedo'
import { Root } from './Root'

vi.mock('@/api/daemon', () => ({ isDesktop: vi.fn() }))
vi.mock('@/features/workspace/DesktopApp', () => ({ DesktopApp: () => <textarea aria-label="desktop" /> }))
vi.mock('./App.tsx', () => ({ default: () => <textarea aria-label="preview" /> }))
vi.mock('@/features/editing/undoRedo', () => ({ installUndoRedo: vi.fn(() => () => undefined) }))

beforeEach(() => { vi.clearAllMocks() })

describe('the window\'s own undo and redo', () => {
  it('is turned on once for the desktop window', () => {
    vi.mocked(isDesktop).mockReturnValue(true)
    render(<Root />)
    expect(installUndoRedo).toHaveBeenCalledExactlyOnceWith(document, navigator.userAgent)
  })

  it('is turned on once for the browser preview', () => {
    vi.mocked(isDesktop).mockReturnValue(false)
    render(<Root />)
    expect(installUndoRedo).toHaveBeenCalledExactlyOnceWith(document, navigator.userAgent)
  })
})
