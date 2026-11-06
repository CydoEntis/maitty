import { render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { Markdown } from './Markdown'

const TABLE = '| Name | Qty | Note |\n| :--- | ---: | :---: |\n| `a` | **1** | x |\n| b | 22 | y |'

describe('a table in a message', () => {
  it('is a table with column headers and one cell per column', () => {
    render(<Markdown text={TABLE} />)

    const table = screen.getByRole('table')
    expect(within(table).getAllByRole('columnheader').map((header) => header.textContent)).toEqual(['Name', 'Qty', 'Note'])
    expect(within(table).getAllByRole('row')).toHaveLength(3)
    expect(within(table).getAllByRole('cell').map((cell) => cell.textContent)).toEqual(['a', '1', 'x', 'b', '22', 'y'])
  })

  it('reads inline markdown in a cell and aligns a column as asked', () => {
    render(<Markdown text={TABLE} />)

    expect(screen.getByText('1').tagName).toBe('STRONG')
    expect(screen.getByText('a').tagName).toBe('CODE')
    expect(screen.getByRole('columnheader', { name: 'Qty' })).toHaveClass('text-right')
    expect(screen.getByRole('columnheader', { name: 'Note' })).toHaveClass('text-center')
    expect(screen.getByRole('columnheader', { name: 'Name' })).toHaveClass('text-left')
  })

  it('scrolls inside a named region that the keyboard can reach, so a wide table never widens the message', () => {
    render(<Markdown text={TABLE} />)

    const region = screen.getByRole('region', { name: 'Table' })
    expect(region).toHaveAttribute('tabindex', '0')
    expect(region).toHaveClass('overflow-x-auto', 'max-w-full')
    expect(within(region).getByRole('table')).toBeInTheDocument()
  })

  it('shows markup in a cell as text', () => {
    const { container } = render(<Markdown text={'| h |\n|---|\n| <img src=x onerror=alert(1)> |'} />)

    expect(container.querySelector('img')).toBeNull()
    expect(screen.getByRole('cell')).toHaveTextContent('<img src=x onerror=alert(1)>')
  })

  it('leaves text around it as paragraphs', () => {
    render(<Markdown text={'Before.\n\n' + TABLE + '\n\nAfter.'} />)

    expect(screen.getByText('Before.')).toBeInTheDocument()
    expect(screen.getByText('After.')).toBeInTheDocument()
    expect(screen.getAllByRole('table')).toHaveLength(1)
  })
})
