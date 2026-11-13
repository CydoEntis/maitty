import { describe, expect, it } from 'vitest'
import { linesOfScreen } from './screenText'

const ESC = String.fromCharCode(27)

describe('the text of a screen the daemon sent', () => {
  it('skips a control sequence it cannot read and keeps the lines after it', () => {
    expect(linesOfScreen(`Pick one\r\n${ESC}[ \r\n❯ 1. Yes`).map((line) => line.trim())).toEqual(['Pick one', '', '❯ 1. Yes'])
  })

  it('skips a title that is never closed and keeps the lines after it', () => {
    expect(linesOfScreen(`${ESC}]0;a title\r\nPick one`)).toEqual(['', 'Pick one'])
  })

  it('leaves nothing behind of an escape that takes a letter after its mark', () => {
    expect(linesOfScreen(`${ESC}(BPick one${ESC}7${ESC}8`)).toEqual(['Pick one'])
  })
})
