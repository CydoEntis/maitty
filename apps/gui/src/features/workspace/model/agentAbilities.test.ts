import { describe, expect, it } from 'vitest'
import { CODEX_START_MODES, PICKABLE_MODES } from '@/features/chat/chatMeta'
import { ANSWER_KEYS } from '@/features/chat/permissionAsk'
import { abilitiesOf, hasAbility, type StructuredAbility } from './agentAbilities'
import { moveRefusal } from './moveToWorktree'
import { CHAT_AGENTS } from './chatView'
import type { SessionView } from './session'

describe('abilitiesOf', () => {
  it('gives Claude its own commands, Shift+Tab modes, a structured chat, moves, verified keys, live model chips, `!` lines, saved conversations, model discovery and hooked asks', () => {
    expect(abilitiesOf('claude')).toEqual({
      slashCommands: { list: 'agent', skillsAfterDollar: false, enterAfterPause: false },
      permissionModes: { modes: PICKABLE_MODES, switchBy: 'shift-tab' },
      structuredChat: true,
      moveToWorktree: true,
      answerKeys: { approve: '1', deny: '\x1b' },
      // The sidebar and the chat disagree today: the chat declines with Esc, the sidebar types `n` (and `a`),
      // which Claude's menu ignores. Pinned as each behaves, not reconciled.
      sidebarAnswerKeys: { y: '1', a: 'a', n: 'n' },
      changeAgent: true,
      bangToTerminal: true,
      resumeSaved: true,
      discoversModels: true,
      hookedAsks: true,
    })
  })

  it('gives Codex the window\'s commands with $ skills, command-switched modes, a structured chat, moves, saved conversations and model discovery, but no verified keys, `!` lines or hooked asks', () => {
    expect(abilitiesOf('codex')).toEqual({
      slashCommands: { list: 'window', skillsAfterDollar: true, enterAfterPause: true },
      permissionModes: { modes: CODEX_START_MODES, switchBy: 'command' },
      structuredChat: true,
      moveToWorktree: true,
      answerKeys: null,
      // The chat sends nothing to Codex's menu (its keys are unverified), while the sidebar types the bare letters.
      // Pinned as each behaves, not reconciled.
      sidebarAnswerKeys: { y: 'y', a: 'a', n: 'n' },
      changeAgent: false,
      bangToTerminal: false,
      resumeSaved: true,
      discoversModels: true,
      hookedAsks: false,
    })
  })

  it('gives Gemini only `!` lines, and the sidebar\'s bare letters', () => {
    expect(abilitiesOf('gemini')).toEqual({
      slashCommands: null,
      permissionModes: null,
      structuredChat: false,
      moveToWorktree: false,
      answerKeys: null,
      sidebarAnswerKeys: { y: 'y', a: 'a', n: 'n' },
      changeAgent: false,
      bangToTerminal: true,
      resumeSaved: false,
      discoversModels: false,
      hookedAsks: false,
    })
  })

  it('treats another agent, a plain shell and a differently spelled name as Gemini is treated, without its `!` lines', () => {
    for (const name of ['opencode', 'Claude', ' codex', null, undefined, 'toString']) {
      expect(abilitiesOf(name)).toEqual({ ...abilitiesOf('gemini'), bangToTerminal: false })
    }
  })

  it('allows a move to a worktree exactly where moveRefusal does', () => {
    for (const name of ['claude', 'codex', 'gemini', 'opencode']) {
      const session = { process: name, agentSession: 'abc', status: 'idle' } as SessionView
      expect(abilitiesOf(name).moveToWorktree).toBe(moveRefusal(session) === null)
    }
  })

  it('answers Claude\'s menu with the keys the chat types', () => {
    expect(abilitiesOf('claude').answerKeys).toBe(ANSWER_KEYS)
  })

  it('gives the abilities behind a structured agent\'s daemon calls to Claude and Codex only', () => {
    const structured: readonly StructuredAbility[] = ['structuredChat', 'resumeSaved', 'discoversModels']
    for (const ability of structured) {
      expect(CHAT_AGENTS.filter((agent) => hasAbility(agent, ability))).toEqual(['claude', 'codex'])
    }
  })
})
