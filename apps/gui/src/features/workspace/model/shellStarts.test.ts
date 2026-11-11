import { beforeEach, describe, expect, it } from 'vitest'
import { rememberShellStart, startedAsShell } from './shellStarts'

describe('the sessions opened as a plain terminal', () => {
  beforeEach(() => { localStorage.clear() })

  it('knows a session that was opened as a terminal, and no other', () => {
    rememberShellStart('7')
    expect(startedAsShell('7')).toBe(true)
    expect(startedAsShell('8')).toBe(false)
  })

  it('still knows it after the window is opened again', () => {
    rememberShellStart('7')
    expect(JSON.parse(localStorage.getItem('maitty.gui.shellStarts.v1') ?? '[]')).toEqual(['7'])
  })

  it('treats unreadable storage as nothing remembered', () => {
    localStorage.setItem('maitty.gui.shellStarts.v1', 'not json')
    expect(startedAsShell('7')).toBe(false)
  })
})
