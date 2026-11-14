import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { Button, IconButton } from './button'
import { Card, CardFoot, CardHead } from './card'
import { Fold } from './fold'
import { Progress } from './progress'
import { StatusMark, type MarkState } from './status-mark'
import { FileBadge, Tag, typeTone } from './tag'
import { ToastCard } from './toast-card'

describe('Button', () => {
  it('draws its kind and size, and shows the key that also presses it', () => {
    render(<Button variant="needs" size="sm" keyHint="y">Yes once</Button>)
    const button = screen.getByRole('button', { name: /^Yes once/ })
    expect(button).toHaveAttribute('data-variant', 'needs')
    expect(button).toHaveAttribute('data-size', 'sm')
    expect(button.querySelector('kbd')).toHaveTextContent('y')
    expect(button).toHaveAccessibleName('Yes once')
    expect(button).toHaveAttribute('aria-keyshortcuts', 'y')
  })

  it('is the primary kind at the medium size unless told otherwise', () => {
    render(<Button>Save</Button>)
    expect(screen.getByRole('button', { name: 'Save' })).toHaveAttribute('data-variant', 'primary')
    expect(screen.getByRole('button', { name: 'Save' })).toHaveAttribute('data-size', 'md')
  })

  it('takes keyboard focus and presses with Enter and Space, but not while disabled', async () => {
    const user = userEvent.setup()
    const press = vi.fn()
    render(<><Button onClick={press}>Go</Button><Button disabled onClick={press}>Stopped</Button></>)
    await user.tab()
    expect(screen.getByRole('button', { name: 'Go' })).toHaveFocus()
    await user.keyboard('{Enter}')
    await user.keyboard(' ')
    expect(press).toHaveBeenCalledTimes(2)
    await user.tab()
    expect(screen.getByRole('button', { name: 'Stopped' })).not.toHaveFocus()
  })

  it('names an icon button by its label, which is also its tooltip', () => {
    render(<IconButton label="Copy"><svg /></IconButton>)
    const button = screen.getByRole('button', { name: 'Copy' })
    expect(button).toHaveAttribute('title', 'Copy')
    expect(button).toHaveAttribute('data-variant', 'ghost')
  })
})

describe('Card', () => {
  it('keeps its head and foot in order and can be reached by keyboard when it is interactive', async () => {
    const user = userEvent.setup()
    render(<Card tone="needs" interactive tabIndex={0} aria-label="Waiting"><CardHead>Run a command</CardHead><CardFoot><Button>Allow</Button></CardFoot></Card>)
    const card = screen.getByLabelText('Waiting')
    expect(card.className).toContain('border-(--needs)')
    await user.tab()
    expect(card).toHaveFocus()
    await user.tab()
    expect(screen.getByRole('button', { name: 'Allow' })).toHaveFocus()
  })
})

describe('StatusMark', () => {
  it.each<[MarkState, string]>([['running', 'running'], ['done', 'done'], ['failed', 'failed'], ['stopped', 'stopped'], ['waiting', 'waiting'], ['needs', 'needs you']])('reads %s by its mark and word, not by colour', (state, word) => {
    render(<StatusMark state={state} word />)
    expect(screen.getByRole('img', { name: word })).toHaveAttribute('data-state', state)
    expect(screen.getByText(word, { selector: 'span:not([role])' })).toBeInTheDocument()
  })

  it('draws the needs mark as ! and the spinner the caller picked while running', () => {
    render(<><StatusMark state="needs" /><StatusMark state="running" spinner={<i data-testid="picked" />} /></>)
    expect(screen.getByRole('img', { name: 'needs you' })).toHaveTextContent('!')
    expect(screen.getByTestId('picked')).toBeInTheDocument()
  })
})

describe('Tag and FileBadge', () => {
  it('colours a tag by what it says', () => {
    render(<><Tag tone="err">exit 1</Tag><Tag>Modified</Tag></>)
    expect(screen.getByText('exit 1').className).toContain('text-(--err)')
    expect(screen.getByText('Modified').className).toContain('text-(--fg2)')
  })

  it('gives each file type its own tone and never the needs colour', () => {
    expect(['TS', 'TSX', 'JSON', 'YAML', 'SQL', 'ENV', 'MD', 'LOG'].map(typeTone)).toEqual([
      'var(--tag-ts)', 'var(--tag-tsx)', 'var(--tag-data)', 'var(--tag-data)', 'var(--tag-sql)', 'var(--tag-env)', 'var(--tag-text)', 'var(--tag-text)',
    ])
  })

  it('shows the folder dimmed, keeps the whole path as its title, and opens from the keyboard', async () => {
    const user = userEvent.setup()
    const open = vi.fn()
    render(<FileBadge path="src/auth/session.ts" tag="TS" line={12} onOpen={open} />)
    const badge = screen.getByRole('button', { name: /session\.ts/ })
    expect(badge).toHaveAttribute('title', 'src/auth/session.ts')
    expect(screen.getByText('src/auth/')).toBeInTheDocument()
    expect(screen.getByText(':12')).toBeInTheDocument()
    await user.tab()
    await user.keyboard('{Enter}')
    expect(open).toHaveBeenCalledOnce()
  })

  it('is plain text without an action, for use inside a row that is a button', () => {
    render(<FileBadge path="README.md" tag="MD" />)
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
    expect(screen.getByTitle('README.md')).toBeInTheDocument()
  })
})

describe('Fold, Progress and Toast', () => {
  it('says what it would show while closed and Hide while open, and toggles by keyboard', async () => {
    const user = userEvent.setup()
    const toggle = vi.fn()
    const { rerender } = render(<Fold open={false} onToggle={toggle} more="Show 12 more lines" />)
    const fold = screen.getByRole('button', { name: 'Show 12 more lines' })
    expect(fold).toHaveAttribute('aria-expanded', 'false')
    await user.tab()
    await user.keyboard('{Enter}')
    expect(toggle).toHaveBeenCalledOnce()
    rerender(<Fold open onToggle={toggle} more="Show 12 more lines" />)
    expect(screen.getByRole('button', { name: 'Hide' })).toHaveAttribute('aria-expanded', 'true')
  })

  it('reports progress as a value of a maximum', () => {
    render(<Progress value={2} max={5} label="Steps passed" />)
    const bar = screen.getByRole('progressbar', { name: 'Steps passed' })
    expect(bar).toHaveAttribute('aria-valuenow', '2')
    expect(bar).toHaveAttribute('aria-valuemax', '5')
  })

  it('announces a toast politely with its actions beside it', () => {
    render(<ToastCard actions={<Button size="sm" variant="soft">Open</Button>}>Ship partial refunds needs you</ToastCard>)
    expect(screen.getByRole('status')).toHaveTextContent('Ship partial refunds needs you')
    expect(screen.getByRole('button', { name: 'Open' })).toBeInTheDocument()
  })
})
