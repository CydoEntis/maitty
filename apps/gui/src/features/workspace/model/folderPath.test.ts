import { describe, expect, it } from 'vitest'
import { parentOf, splitTyped, withoutTrailingSlash } from './folderPath'

describe('the folder arithmetic', () => {
  it('goes up a level, and stops at the home folder and the root', () => {
    expect(parentOf('~/code/maitty')).toBe('~/code')
    expect(parentOf('~/code/')).toBe('~')
    expect(parentOf('~')).toBe('~')
    expect(parentOf('/srv/app')).toBe('/srv')
    expect(parentOf('/srv')).toBe('/')
    expect(parentOf('/')).toBe('/')
  })

  it('splits what is typed into the folder it is in and the part after it', () => {
    expect(splitTyped('~/code/ma')).toEqual({ folder: '~/code/', part: 'ma' })
    expect(splitTyped('~/code/')).toEqual({ folder: '~/code/', part: '' })
    expect(splitTyped('~')).toEqual({ folder: '~/', part: '' })
    expect(withoutTrailingSlash('~/code/')).toBe('~/code')
    expect(withoutTrailingSlash('/')).toBe('/')
  })
})
