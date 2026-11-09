import { describe, expect, it } from 'vitest'
import { withNickname } from './machineNicknames'

describe('machine nicknames', () => {
  it('names a machine by its lower-cased real name', () => {
    expect(withNickname({}, 'Build-Box', '  Builder ')).toEqual({ 'build-box': 'Builder' })
  })

  it('takes the name away when it is emptied', () => {
    expect(withNickname({ 'build-box': 'Builder', dev: 'Dev' }, 'build-box', ' ')).toEqual({ dev: 'Dev' })
  })
})
