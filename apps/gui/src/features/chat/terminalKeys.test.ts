import { describe, expect, it } from 'vitest'
import { partsOfPrompt, promptOfScreen } from './terminalKeys'

const ESC = String.fromCharCode(27)

/**
 * A Claude question menu as the daemon sends a screen: the menu's lines drawn into the daemon's own terminal
 * model at 80 columns and written back out by its formatter, so the colour codes, the cursor jumps over blank
 * lines and the rule above "Chat about this" are the ones the desktop app receives.
 */
const QUESTION_SCREEN = '\u001b[0m\u001b[2J\u001b[H\u001b[?25h\u001b[m\u001b[H\u001b[J\u001b[38;5;153m❯\u001b[m pick a database for the service\u001b[3;1H● I need one decision before I start.\u001b[5;1H\u001b[38;5;246m────────────────────────────────────────────────────────────────────────────────\r\n\u001b[m \u001b[38;5;153m☐ Database\u001b[8;1H\u001b[39;1mWhich database should the service use?\u001b[10;1H\u001b[38;5;153;22m❯ 1. SQLite\r\n\u001b[m     \u001b[38;5;246mOne file, no server to run.\r\n\u001b[m  2. Postgres\r\n     \u001b[38;5;246mA server, with real concurrency.\r\n\u001b[m  3. MySQL\r\n     \u001b[38;5;246mWhat the hosting plan already includes.\r\n\u001b[m  4. DuckDB\r\n     \u001b[38;5;246mFast for reports, weak for many writers.\r\n\u001b[m  5. Type something.\r\n\u001b[38;5;246m────────────────────────────────────────────────────────────────────────────────\r\n\u001b[m  6. Chat about this\u001b[22;1H\u001b[38;5;246mEnter to select · ↑/↓ to navigate · Esc to cancel\u001b[m\u001b>\u001b[?1l\u001b[?2004l'

const OPTIONS = ['1. SQLite', '2. Postgres', '3. MySQL', '4. DuckDB', '5. Type something.', '6. Chat about this']

describe('the question an agent draws in its terminal, as the chat shows it', () => {
  it('shows no terminal escape codes', () => {
    const shown = promptOfScreen(QUESTION_SCREEN)
    expect(shown).not.toContain(ESC)
    expect(shown).not.toMatch(/\[[\d;?]*[a-zA-Z]/)
  })

  it('shows the question, every option and the one now selected, not only the footer under the last rule', () => {
    const lines = promptOfScreen(QUESTION_SCREEN).split('\n')
    expect(lines).toContain('Which database should the service use?')
    expect(lines).toContain('❯ 1. SQLite')
    for (const option of OPTIONS.slice(1)) expect(lines).toContain(`  ${option}`)
    expect(lines).toContain('     One file, no server to run.')
    expect(lines).toContain('Enter to select · ↑/↓ to navigate · Esc to cancel')
  })

  it('leaves out the conversation above the menu and the rules that frame it', () => {
    const shown = promptOfScreen(QUESTION_SCREEN)
    expect(shown).not.toContain('I need one decision before I start.')
    expect(shown).not.toContain('────')
  })

  it('still shows the whole menu when the option under the rule is the one selected', () => {
    const moved = QUESTION_SCREEN.replace('❯ 1. SQLite', '  1. SQLite').replace('  6. Chat about this', '❯ 6. Chat about this')
    const lines = promptOfScreen(moved).split('\n')
    expect(lines).toContain('Which database should the service use?')
    expect(lines).toContain('  1. SQLite')
    expect(lines).toContain('❯ 6. Chat about this')
  })

  it('keeps the question and every option of a menu taller than the card', () => {
    const many = Array.from({ length: 30 }, (_, at) => `${at === 0 ? '❯' : ' '} ${String(at + 1)}. Choice ${String(at + 1)}`)
    const shown = promptOfScreen(['earlier conversation', '────────────', 'Which one?', '', ...many, '', 'Enter to select'].join('\n'))
    const { question, options } = partsOfPrompt(shown)
    expect(question).toBe('Which one?')
    expect(options.split('\n')).toEqual(expect.arrayContaining(many))
  })

  it('splits what it shows into the question, which stays, and the options, which scroll', () => {
    const { question, options } = partsOfPrompt(promptOfScreen(QUESTION_SCREEN))
    expect(question).toContain('Which database should the service use?')
    expect(question).not.toContain('SQLite')
    expect(options.startsWith('❯ 1. SQLite')).toBe(true)
    expect(options).toContain('6. Chat about this')
  })

  it('reads a cursor position far outside any screen without drawing that many lines', () => {
    const shown = promptOfScreen(`Pick one${ESC}[999999999;1Hfar away${ESC}[999999999Cfarther`)
    expect(shown).toContain('far away')
    expect(shown.length).toBeLessThan(10_000)
  })
})
