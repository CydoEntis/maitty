import { describe, expect, it } from 'vitest'
import { remotePathText, routeDrop, typablePath } from './dropRoute'

describe('where dropped files go', () => {
  it('types the paths into a pane of this computer, quoted for its shell', () => {
    expect(routeDrop('t3', ['/home/me/my file.png', '/tmp/a.log'], false)).toEqual({ kind: 'paste', text: '/home/me/my\\ file.png /tmp/a.log ' })
    expect(routeDrop('t3', ['C:\\Users\\me\\my file.png'], true)).toEqual({ kind: 'paste', text: '"C:\\Users\\me\\my file.png" ' })
  })

  it('sends the files to the machine of a pane on another one', () => {
    expect(routeDrop('build/t12', ['/home/me/a.log', '/home/me/b.zip'], false)).toEqual({ kind: 'send', machine: 'build', paths: ['/home/me/a.log', '/home/me/b.zip'] })
  })

  it('does nothing for a drop with no path in it', () => {
    expect(routeDrop('t3', [], false)).toBeNull()
    expect(routeDrop('build/t12', [''], false)).toBeNull()
  })

  it('takes ten files for another machine and refuses eleven whole', () => {
    const files = (count: number) => Array.from({ length: count }, (_, n) => `/home/me/f${String(n)}`)
    expect(routeDrop('build/t12', files(10), false)).toMatchObject({ kind: 'send' })
    expect(routeDrop('build/t12', files(11), false)).toEqual({ kind: 'refuse', message: 'At most 10 files can be sent at a time; 11 were dropped.' })
  })

  it('has no limit on a pane of this computer, which only types paths', () => {
    const files = Array.from({ length: 30 }, (_, n) => `/home/me/f${String(n)}`)
    expect(routeDrop('t3', files, false)).toMatchObject({ kind: 'paste' })
  })
})

describe('a path another machine names', () => {
  it('is typed quoted, with a space after, when it is an absolute plain path', () => {
    expect(remotePathText('/home/me/.cache/maitty/pastes/paste-1-ab-a_b.log')).toBe('/home/me/.cache/maitty/pastes/paste-1-ab-a_b.log ')
    expect(remotePathText('/home/my user/p.log')).toBe('/home/my\\ user/p.log ')
  })

  it('is refused when it is not absolute, is too long or holds anything that is not text', () => {
    expect(typablePath('relative/p.log')).toBeNull()
    expect(typablePath('~/p.log')).toBeNull()
    expect(typablePath(`/${'a'.repeat(4096)}`)).toBeNull()
    for (const hidden of ['\u001b[31m', '\n', '\u0000', '\u202e', '\u200b', '\ufeff', '\u0085']) {
      expect(typablePath(`/home/me/p${hidden}.log`), JSON.stringify(hidden)).toBeNull()
      expect(remotePathText(`/home/me/p${hidden}.log`)).toBeNull()
    }
  })
})
