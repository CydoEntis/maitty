import { describe, expect, it } from 'vitest'
import { commandText, isSlashLine, matchCommands, slashQuery, withTerminalCommands, type SlashCommand } from './slashCommands'

const command = (name: string, aliases: string[] = []): SlashCommand => ({ name, description: '', hint: '', aliases })
const names = (commands: SlashCommand[]) => commands.map((entry) => entry.name)

describe('a slash command in the message box', () => {
  it('is being typed while the box holds a slash and a name, and no longer once a space ends the name', () => {
    expect(slashQuery('/')).toBe('')
    expect(slashQuery('/comp')).toBe('comp')
    expect(slashQuery('/developer:pr')).toBe('developer:pr')
    expect(slashQuery('/compact ')).toBeNull()
    expect(slashQuery('look at /tmp')).toBeNull()
    expect(slashQuery(' /compact')).toBeNull()
    expect(slashQuery('')).toBeNull()
  })

  it('is told from a message by being one line that starts with a slash', () => {
    expect(isSlashLine('/compact')).toBe(true)
    expect(isSlashLine('/model opus')).toBe(true)
    expect(isSlashLine('/')).toBe(false)
    expect(isSlashLine('/ compact')).toBe(false)
    expect(isSlashLine('/compact\nand then this')).toBe(false)
    expect(isSlashLine('see /tmp/x')).toBe(false)
  })

  it('lists every command by name when nothing is typed', () => {
    expect(names(matchCommands([command('model'), command('clear'), command('compact')], ''))).toEqual(['clear', 'compact', 'model'])
  })

  it('puts the exact name first, then names that start with what was typed, then plugin commands and other names, then names that hold it', () => {
    const all = [command('recompact'), command('developer:compact-notes'), command('compact-all'), command('compact'), command('shrink', ['compacter']), command('clear')]
    expect(names(matchCommands(all, 'COMPACT'))).toEqual(['compact', 'compact-all', 'developer:compact-notes', 'shrink', 'recompact'])
    expect(matchCommands(all, 'zzz')).toEqual([])
  })

  it('adds the terminal\'s commands to the agent\'s own list, except one the agent already lists by name or other name', () => {
    const listed = [command('compact'), command('settings', ['config'])]
    const terminal = [command('help'), command('config'), command('compact')]
    expect(names(withTerminalCommands(listed, terminal))).toEqual(['compact', 'settings', 'help'])
  })

  it('leaves its name and a space in the box when picked', () => {
    expect(commandText(command('developer:pr'))).toBe('/developer:pr ')
  })
})
