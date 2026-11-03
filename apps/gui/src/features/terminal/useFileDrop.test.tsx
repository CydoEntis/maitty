import { renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useFileDrop } from './useFileDrop'

type DragEvent = { payload: { type: 'enter' | 'over' | 'drop' | 'leave'; paths?: string[]; position?: { x: number; y: number } } }

const shell = vi.hoisted(() => ({
  isTauri: true,
  handler: null as null | ((event: DragEvent) => void),
  unlisten: vi.fn(),
}))

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => shell.isTauri }))
vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: (handler: (event: DragEvent) => void) => {
      shell.handler = handler
      return Promise.resolve(shell.unlisten)
    },
  }),
}))

const flush = () => new Promise((resolve) => setTimeout(resolve, 0))

beforeEach(() => {
  shell.isTauri = true
  shell.handler = null
  shell.unlisten.mockClear()
})

describe('file drop', () => {
  it('reports dropped files with the pointer in CSS pixels', async () => {
    Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: 2 })
    const onDrop = vi.fn()
    renderHook(() => { useFileDrop(onDrop) })
    await flush()

    shell.handler?.({ payload: { type: 'drop', paths: ['/a/b.png'], position: { x: 200, y: 100 } } })

    expect(onDrop).toHaveBeenCalledWith(['/a/b.png'], { x: 100, y: 50 })
  })

  it('ignores a file being dragged over the window until it is dropped', async () => {
    const onDrop = vi.fn()
    renderHook(() => { useFileDrop(onDrop) })
    await flush()

    shell.handler?.({ payload: { type: 'over', position: { x: 1, y: 1 } } })
    shell.handler?.({ payload: { type: 'leave' } })

    expect(onDrop).not.toHaveBeenCalled()
  })

  it('stops listening when the window closes', async () => {
    const { unmount } = renderHook(() => { useFileDrop(vi.fn()) })
    await flush()
    unmount()
    expect(shell.unlisten).toHaveBeenCalledOnce()
  })

  it('listens to nothing outside the desktop app', async () => {
    shell.isTauri = false
    renderHook(() => { useFileDrop(vi.fn()) })
    await flush()
    expect(shell.handler).toBeNull()
  })
})
