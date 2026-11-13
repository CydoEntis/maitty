import { describe, expect, it } from 'vitest'
import { SHARED_CHECKOUT_REFUSAL, sharedCheckoutRefusal } from './sharedCheckout'
import type { SessionView } from './session'

const MAIN = 'C:\\dev\\app'
const LINKED = 'C:\\dev\\app-trees\\fix'

const session = (patch: Partial<SessionView>) => ({
  id: 't2', machine: 0, process: 'claude', ended: false, cwd: MAIN, checkout: MAIN, ...patch,
}) as SessionView

const restoring = { machine: 0, checkout: MAIN }
const own = new Set(['t1'])

describe('sharedCheckoutRefusal', () => {
  it('refuses when another agent works in the same checkout, however its path is written', () => {
    expect(sharedCheckoutRefusal([session({ cwd: 'c:/dev/app', checkout: 'c:/dev/app/' })], restoring, own)).toBe(SHARED_CHECKOUT_REFUSAL)
  })

  it('refuses when the other checkout is written as a raw canonical Windows path', () => {
    expect(sharedCheckoutRefusal([session({ cwd: '\\\\?\\C:\\dev\\app', checkout: '\\\\?\\C:\\dev\\app' })], restoring, own)).toBe(SHARED_CHECKOUT_REFUSAL)
  })

  it('tells apart POSIX folders that differ only by case', () => {
    const posix = { machine: 0, checkout: '/home/casey/app' }
    expect(sharedCheckoutRefusal([session({ cwd: '/home/casey/App', checkout: '/home/casey/App' })], posix, own)).toBeNull()
    expect(sharedCheckoutRefusal([session({ cwd: '/home/casey/App/src', checkout: null })], posix, own)).toBeNull()
    expect(sharedCheckoutRefusal([session({ cwd: '/home/casey/app/', checkout: '/home/casey/app/' })], posix, own)).toBe(SHARED_CHECKOUT_REFUSAL)
  })

  it('refuses when another agent works in a subfolder of the checkout', () => {
    expect(sharedCheckoutRefusal([session({ cwd: `${MAIN}\\crates\\core` })], restoring, own)).toBe(SHARED_CHECKOUT_REFUSAL)
  })

  it('refuses for a session the daemon gave no checkout for when its folder is inside this one', () => {
    expect(sharedCheckoutRefusal([session({ cwd: `${MAIN}\\src`, checkout: null })], restoring, own)).toBe(SHARED_CHECKOUT_REFUSAL)
  })

  it('allows a session in a different checkout', () => {
    expect(sharedCheckoutRefusal([session({ cwd: 'C:\\dev\\website', checkout: 'C:\\dev\\website' })], restoring, own)).toBeNull()
    expect(sharedCheckoutRefusal([session({ cwd: 'C:\\dev\\application', checkout: 'C:\\dev\\application' })], restoring, own)).toBeNull()
  })

  it('allows another linked worktree of the same repository: its files are its own', () => {
    expect(sharedCheckoutRefusal([session({ cwd: LINKED, checkout: LINKED })], restoring, own)).toBeNull()
    expect(sharedCheckoutRefusal([session({ cwd: `${MAIN}\\.claude\\worktrees\\x`, checkout: `${MAIN}\\.claude\\worktrees\\x` })], restoring, own)).toBeNull()
  })

  it('allows a closed session', () => {
    expect(sharedCheckoutRefusal([session({ ended: true })], restoring, own)).toBeNull()
  })

  it('allows a session on another machine', () => {
    expect(sharedCheckoutRefusal([session({ machine: 2 })], restoring, own)).toBeNull()
  })

  it('leaves out the session itself, its own terminals and plain shells', () => {
    expect(sharedCheckoutRefusal([session({ id: 't1' })], restoring, own)).toBeNull()
    expect(sharedCheckoutRefusal([session({ process: 'shell' })], restoring, own)).toBeNull()
  })

  it('refuses nothing when the checkout to restore is not known', () => {
    expect(sharedCheckoutRefusal([session({})], { machine: 0, checkout: null }, own)).toBeNull()
  })
})
