import { describe, expect, it } from 'vitest'
import type { FolderListing } from '@/api/folders'
import { completionFor, effectiveStartView, folderName, isPathLike, matchingProjects, viewLockReason } from './newSession'

const listing = (names: string[]): FolderListing => ({ path: '~', exists: true, entries: names.map((name) => ({ name, git: null, has_children: false })), truncated: false })

describe('the view toggle lock', () => {
  it('locks to the terminal for a shell and for another machine, and says why', () => {
    expect(viewLockReason('terminal', 0)).toBe('Shells always open in Terminal')
    expect(viewLockReason('claude', 2)).toBe('Chat runs on this computer')
    expect(viewLockReason('claude', 0)).toBeNull()
    expect(effectiveStartView('chat', 'terminal', 0)).toBe('terminal')
    expect(effectiveStartView('chat', 'claude', 2)).toBe('terminal')
    expect(effectiveStartView('chat', 'codex', 0)).toBe('chat')
  })
})

describe('completing a project', () => {
  const projects = ['/home/u/AeVoxGrowth', '/home/u/pacman']
  it('completes a name from the known projects and offers the whole folder', () => {
    expect(completionFor('Ae', projects, null)).toEqual({ rest: 'VoxGrowth', folder: '/home/u/AeVoxGrowth' })
    expect(completionFor('pac', projects, null)).toEqual({ rest: 'man', folder: '/home/u/pacman' })
    expect(completionFor('zzz', projects, null)).toBeNull()
    expect(completionFor('', projects, null)).toBeNull()
  })
  it('completes a path from the folders that exist, hiding dotfolders until a dot is typed', () => {
    const inside = listing(['.cache', 'code', 'notes'])
    expect(completionFor('~/co', [], inside)).toEqual({ rest: 'de', folder: '~/code/' })
    expect(completionFor('~/.c', [], inside)).toEqual({ rest: 'ache', folder: '~/.cache/' })
    expect(completionFor('~/', [], inside)?.folder).toBe('~/code/')
    expect(completionFor('~/code', [], inside)).toBeNull()
    expect(completionFor('~/x', [], { ...inside, exists: false })).toBeNull()
  })
  it('knows a path from a name, and names a folder by its last segment', () => {
    expect(isPathLike('~/a')).toBe(true)
    expect(isPathLike('/a')).toBe(true)
    expect(isPathLike('a')).toBe(false)
    expect(folderName('/home/u/pacman/')).toBe('pacman')
    expect(folderName('~')).toBe('~')
  })
  it('ranks name prefixes above other matches', () => {
    expect(matchingProjects('man', ['/a/pacman', '/a/manual', '/a/other'])).toEqual(['/a/manual', '/a/pacman'])
    expect(matchingProjects('', ['/a', '/b'])).toEqual(['/a', '/b'])
  })
})
