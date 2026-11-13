import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { SearchHit } from '@/features/find/api'
import type { PaletteItem } from '../model/palette'
import { CommandPalette } from './CommandPalette'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }))

const openSession = vi.fn()
const outputItem = (hit: SearchHit): PaletteItem => ({ id: `output-${String(hit.term)}:${String(hit.line)}`, label: `pane ${String(hit.term)}`, detail: hit.text, kind: 'Output', run: () => { openSession(hit.term) } })
const items: PaletteItem[] = [{ id: 'build', label: 'Build the docs', kind: 'Action', run: () => undefined }]

function renderPalette() {
  render(<CommandPalette items={items} outputItem={outputItem} onClose={() => undefined} />)
  return screen.getByLabelText('Search the palette')
}

const options = () => screen.getAllByRole('option').map((option) => option.textContent)
const PAST_DEBOUNCE_MS = 400
const pause = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

beforeEach(() => {
  vi.mocked(isTauri).mockReturnValue(true)
  vi.mocked(invoke).mockReset()
  openSession.mockReset()
})

describe('Command palette output search', () => {
  it('lists what sessions printed under In sessions, after the other results, and opens the session', async () => {
    vi.mocked(invoke).mockResolvedValue([{ term: 7, line: 3, text: 'build failed' }])
    const input = renderPalette()
    fireEvent.change(input, { target: { value: 'build' } })
    await screen.findByText('In sessions')
    expect(options()).toEqual(['Build the docsAction', 'pane 7build failed'])
    fireEvent.click(screen.getByText('pane 7'))
    expect(openSession).toHaveBeenCalledWith(7)
  })

  it('waits for a pause in typing and asks only for the last query', async () => {
    vi.mocked(invoke).mockResolvedValue([])
    const input = renderPalette()
    for (const text of ['bui', 'buil', 'build']) fireEvent.change(input, { target: { value: text } })
    await waitFor(() => { expect(invoke).toHaveBeenCalledTimes(1) })
    expect(invoke).toHaveBeenCalledWith('search_panes', { query: 'build' })
  })

  it('never asks for fewer than three characters', async () => {
    const input = renderPalette()
    fireEvent.change(input, { target: { value: 'bu' } })
    await pause(PAST_DEBOUNCE_MS)
    expect(invoke).not.toHaveBeenCalled()
  })

  it('drops an answer that comes back after the query changed', async () => {
    let answerOld: (hits: unknown) => void = () => undefined
    vi.mocked(invoke).mockImplementationOnce(() => new Promise((resolve) => { answerOld = resolve })).mockResolvedValueOnce([])
    const input = renderPalette()
    fireEvent.change(input, { target: { value: 'old' } })
    await waitFor(() => { expect(invoke).toHaveBeenCalledTimes(1) })
    fireEvent.change(input, { target: { value: 'newer' } })
    answerOld([{ term: 1, line: 1, text: 'old line' }])
    await waitFor(() => { expect(invoke).toHaveBeenCalledTimes(2) })
    expect(screen.queryByText('In sessions')).toBeNull()
  })

  it('runs one search at a time and then only the newest query typed meanwhile', async () => {
    let answerFirst: (hits: unknown) => void = () => undefined
    vi.mocked(invoke).mockImplementationOnce(() => new Promise((resolve) => { answerFirst = resolve })).mockResolvedValue([])
    const input = renderPalette()
    fireEvent.change(input, { target: { value: 'abc' } })
    await waitFor(() => { expect(invoke).toHaveBeenCalledTimes(1) })
    fireEvent.change(input, { target: { value: 'abcd' } })
    await pause(PAST_DEBOUNCE_MS)
    fireEvent.change(input, { target: { value: 'abcde' } })
    await pause(PAST_DEBOUNCE_MS)
    expect(invoke).toHaveBeenCalledTimes(1)
    answerFirst([])
    await waitFor(() => { expect(invoke).toHaveBeenCalledTimes(2) })
    expect(invoke).toHaveBeenLastCalledWith('search_panes', { query: 'abcde' })
  })

  it('lists one hit per session so a busy session cannot fill the list', async () => {
    const busy = Array.from({ length: 14 }, (_, index) => ({ term: 1, line: 20 - index, text: 'build step' }))
    vi.mocked(invoke).mockResolvedValue([...busy, { term: 2, line: 5, text: 'build done' }])
    const input = renderPalette()
    fireEvent.change(input, { target: { value: 'build' } })
    const group = await screen.findByRole('group', { name: 'In sessions' })
    expect(within(group).getAllByRole('option').map((option) => option.textContent)).toEqual(['pane 1build step', 'pane 2build done'])
  })

  it('points the input at the selected option', async () => {
    vi.mocked(invoke).mockResolvedValue([{ term: 7, line: 3, text: 'build failed' }])
    const input = renderPalette()
    fireEvent.change(input, { target: { value: 'build' } })
    await screen.findByRole('group', { name: 'In sessions' })
    fireEvent.keyDown(input, { key: 'ArrowDown' })
    const selected = screen.getAllByRole('option').find((option) => option.getAttribute('aria-selected') === 'true')
    expect(selected?.textContent).toBe('pane 7build failed')
    expect(input.getAttribute('aria-activedescendant')).toBe(selected?.id)
  })

  it('adds nothing and shows no error when the search fails', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('service offline'))
    const input = renderPalette()
    fireEvent.change(input, { target: { value: 'build' } })
    await waitFor(() => { expect(invoke).toHaveBeenCalled() })
    expect(options()).toEqual(['Build the docsAction'])
    expect(screen.queryByRole('alert')).toBeNull()
  })
})
