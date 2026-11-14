import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { DocumentCard } from './DocumentCard'

const document = { title: 'Plan', excerpt: 'Do it.', lineCount: 2, text: '# Plan\nDo it.\nThen the rest.' }

describe('a document the agent wrote', () => {
  it('is read in the chat, whatever folder it was written to, and folds back', async () => {
    render(<DocumentCard path="/home/me/Downloads/plan.md" document={document} />)
    expect(screen.queryByText(/Then the rest/)).not.toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Read' }))
    expect(screen.getByText(/Then the rest/)).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(screen.queryByText(/Then the rest/)).not.toBeInTheDocument()
  })

  it('shows its opening formatted, not as raw Markdown marks run together', () => {
    const notes = { title: 'Notes', excerpt: '- **Clone** a repository\n- **Settings** shows who is signed in', lineCount: 3, text: '# Notes\n- **Clone** a repository\n- **Settings** shows who is signed in' }
    render(<DocumentCard path="notes.md" document={notes} />)
    expect(screen.getAllByRole('listitem')).toHaveLength(2)
    expect(screen.getByText('Clone').tagName).toBe('STRONG')
    expect(screen.queryByText(/\*\*/)).not.toBeInTheDocument()
  })
})
