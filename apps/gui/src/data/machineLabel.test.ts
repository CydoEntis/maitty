import { afterEach, describe, expect, it } from 'vitest'
import { machineLabel, writeMachineNicknames } from './machineNicknames'
import { displayName } from '@/features/workspace/model/session'

afterEach(() => { writeMachineNicknames({}) })

describe('the name shown for a machine', () => {
  it('is the name its owner gave it, in any text that names the machine, and its own name before that', () => {
    expect(machineLabel('100.78.214.15')).toBe('100.78.214.15')
    writeMachineNicknames({ '100.78.214.15': 'Kingdom' })
    expect(machineLabel('100.78.214.15')).toBe('Kingdom')
    expect(machineLabel(null)).toBeNull()
    expect(displayName({ name: 'Bugs', title: 'Bugs', machineName: '100.78.214.15' })).toBe('Kingdom · Bugs')
  })
})
