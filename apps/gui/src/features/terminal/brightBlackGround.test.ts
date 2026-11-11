import { describe, expect, it } from 'vitest'
import { brightBlackGround, type Channels } from './brightBlackGround'

const GROUND: Channels = [9, 34, 41]
const SLAB = '48;2;9;34;41'
const encoder = new TextEncoder()
const decoder = new TextDecoder()

function rewriter(ground: Channels | null = GROUND) {
  const rewrite = brightBlackGround(() => ground)
  return (text: string) => decoder.decode(rewrite(encoder.encode(text)))
}

describe('a bright black background in a terminal', () => {
  it.each([
    ['its own code', '\x1b[100mhello\x1b[0m', `\x1b[${SLAB}mhello\x1b[0m`],
    ['among other attributes', '\x1b[1;100;97mx', `\x1b[1;${SLAB};97mx`],
    ['the 256-colour form', '\x1b[48;5;8mx', `\x1b[${SLAB}mx`],
  ])('is drawn as the theme’s slab when set by %s', (_name, sent, drawn) => {
    expect(rewriter()(sent)).toBe(drawn)
  })

  it.each([
    ['bright black as text', '\x1b[90mdim\x1b[0m'],
    ['a 256-colour text colour whose index is 100', '\x1b[38;5;100mx'],
    ['a true colour with a channel of 100', '\x1b[48;2;100;100;100mx'],
    ['another indexed background', '\x1b[48;5;9mx'],
    ['a sequence that is not a colour change', '\x1b[100Cx\x1b[?100h'],
    ['text with no escapes', 'plain 100m text'],
  ])('leaves %s alone', (_name, sent) => {
    expect(rewriter()(sent)).toBe(sent)
  })

  it('sees a colour change the transport cut in two', () => {
    const rewrite = rewriter()

    expect(rewrite('before\x1b[1;10')).toBe('before')
    expect(rewrite('0mafter')).toBe(`\x1b[1;${SLAB}mafter`)
  })

  it('holds a lone escape at the end of a write, and nothing else', () => {
    const rewrite = rewriter()

    expect(rewrite('a\x1b')).toBe('a')
    expect(rewrite('[0mb')).toBe('\x1b[0mb')
  })

  it('passes everything through when the theme names no slab', () => {
    expect(rewriter(null)('\x1b[100mx')).toBe('\x1b[100mx')
  })
})
