import { afterEach, describe, expect, it } from 'vitest'
import { addressOf, forgetMachines, machineArgs, machineNameOf, parseSessionId, registerMachine, terminalOf, terminalSessionId } from './machines'

afterEach(() => { forgetMachines() })

describe('session ids', () => {
  it('writes a pane of this computer as it always was and a pane of another machine with its name', () => {
    expect(terminalSessionId(12)).toBe('t12')
    expect(terminalSessionId(12, null)).toBe('t12')
    expect(terminalSessionId(12, 'build')).toBe('build/t12')
  })

  it('reads the pane number of any terminal session id and nothing else', () => {
    expect(terminalOf('t12')).toBe(12)
    expect(terminalOf('build/t12')).toBe(12)
    expect(terminalOf('my-box.2/t3')).toBe(3)
    for (const notOurs of ['42', 't', 't1x', '/t1', 'a b/t1', 'build/42', 'build//t1', '-x/t1', 'build/t1/t2']) {
      expect(terminalOf(notOurs), notOurs).toBeNull()
    }
  })

  it('names the machine of an id', () => {
    expect(machineNameOf('t12')).toBeNull()
    expect(machineNameOf('build/t12')).toBe('build')
    expect(machineNameOf('42')).toBeNull()
  })

  it('finds a pane of this computer without being told of any machine', () => {
    expect(parseSessionId('t12')).toEqual({ machine: 0, machineName: null, term: 12 })
    expect(parseSessionId('nonsense')).toBeNull()
  })

  it('finds a pane of another machine only once the host has said which machine that name is', () => {
    expect(parseSessionId('build/t12')).toBeNull()
    registerMachine('Build', 3)
    expect(parseSessionId('build/t12')).toEqual({ machine: 3, machineName: 'build', term: 12 })
  })

  it('refuses to address a command to a machine it has not been told of', () => {
    expect(() => addressOf('build/t12')).toThrow('build is not one of this computer\'s machines')
    expect(addressOf('t12')).toEqual({ machine: 0, machineName: null, term: 12 })
    expect(addressOf('42')).toBeNull()
  })

  it('names the machine to the host only for another machine', () => {
    expect(machineArgs({ machine: 0, machineName: null, term: 1 })).toEqual({})
    expect(machineArgs({ machine: 4, machineName: 'build', term: 1 })).toEqual({ machine: 4 })
  })
})
