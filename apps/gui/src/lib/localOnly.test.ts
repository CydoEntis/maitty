import { describe, expect, it } from 'vitest'
import { assertLocal, guardLocal, guardLocalSession, LOCAL_FEATURES, localOnlyMessage, type LocalFeature } from './localOnly'

const FEATURES = Object.keys(LOCAL_FEATURES) as LocalFeature[]
const remote = { machine: 2, machineName: 'build' }
const here = { machine: 0, machineName: null }

describe('features that need a pane\'s files', () => {
  it.each(FEATURES)('%s is refused for a pane of another machine and allowed here', (feature) => {
    expect(guardLocal(remote, feature)).toBe(`${LOCAL_FEATURES[feature]} is local-only in v1: the files are on build, not on this computer.`)
    expect(guardLocal(here, feature)).toBeNull()
    expect(guardLocal(undefined, feature)).toBeNull()
    expect(() => { assertLocal(remote, feature) }).toThrow(/local-only in v1/)
    expect(() => { assertLocal(here, feature) }).not.toThrow()
  })

  it('says the same as the terminal app, word for word', () => {
    expect(localOnlyMessage('files', 'kingdom')).toBe('Files is local-only in v1: the files are on kingdom, not on this computer.')
    expect(localOnlyMessage('paste-image', 'kingdom')).toBe('Pasting an image is local-only in v1: the files are on kingdom, not on this computer.')
    expect(localOnlyMessage('agent-status', 'kingdom')).toBe('Connecting an agent is local-only in v1: the files are on kingdom, not on this computer.')
  })

  it('reads a session id: a name before the slash is another machine', () => {
    expect(guardLocalSession('build/t12', 'path-links')).toContain('on build')
    expect(guardLocalSession('t12', 'path-links')).toBeNull()
  })
})
