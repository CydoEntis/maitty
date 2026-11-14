import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { SpecialistList } from '@/api/daemon'
import { SpecialistsDialog } from './SpecialistsDialog'

const list: SpecialistList = {
  term: 4,
  project: '/home/me/app',
  truncated: false,
  files: [
    { tool: 'claude', scope: 'project', name: 'reviewer', description: 'Reviews a change', path: '/home/me/app/.claude/agents/reviewer.md', text: '---\nname: reviewer\n---\nYou review.\n', text_cut: false },
    { tool: 'codex', scope: 'home', name: 'builder', description: null, path: '/home/me/.codex/agents/builder.toml', text: 'name = "builder"', text_cut: true },
  ],
}

describe('the specialist agents dialog', () => {
  it("lists the project's and the home folder's agents and opens one read-only", async () => {
    const user = userEvent.setup()
    const load = vi.fn(() => Promise.resolve(list))
    render(<SpecialistsDialog title="~/app" term={4} load={load} onClose={() => {}} />)
    expect(load).toHaveBeenCalledWith(4)
    const project = await screen.findByRole('region', { name: 'This project' })
    expect(project).toHaveTextContent('reviewer')
    expect(project).toHaveTextContent('Claude Code')
    expect(project).toHaveTextContent('Reviews a change')
    expect(screen.getByRole('region', { name: 'Your home folder' })).toHaveTextContent('Codex')
    await user.click(screen.getByRole('button', { name: /builder/ }))
    expect(screen.getByLabelText('builder file')).toHaveTextContent('name = "builder"')
    expect(screen.getByText('/home/me/.codex/agents/builder.toml')).toBeInTheDocument()
    expect(screen.getByText(/only its start is shown/)).toBeInTheDocument()
    expect(screen.queryByRole('textbox')).toBeNull()
    await user.click(screen.getByRole('button', { name: 'Back' }))
    expect(screen.getByRole('region', { name: 'This project' })).toBeInTheDocument()
  })

  it('says where it looked when there are none, and why when it could not look', async () => {
    const { unmount } = render(<SpecialistsDialog title="~/app" term={4} load={() => Promise.resolve({ ...list, files: [] })} onClose={() => {}} />)
    expect(await screen.findByText(/No agent files/)).toBeInTheDocument()
    unmount()
    render(<SpecialistsDialog title="~/app" term={4} load={() => Promise.reject(new Error('No such pane'))} onClose={() => {}} />)
    expect(await screen.findByRole('alert')).toHaveTextContent('No such pane')
  })

  it('closes', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    render(<SpecialistsDialog title="~/app" term={4} load={() => Promise.resolve(list)} onClose={onClose} />)
    await user.click(screen.getByRole('button', { name: 'Close' }))
    expect(onClose).toHaveBeenCalled()
  })
})
