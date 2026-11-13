const ESC_CODE = 27
const BELL_CODE = 7
const DELETE_CODE = 127
const ESC = String.fromCharCode(ESC_CODE)
const BELL = String.fromCharCode(BELL_CODE)
/** A control sequence after its ESC: `[`, its numbers, and the letter that says what it does. */
const CONTROL_SEQUENCE = /\[([\d;:<=>?]*)[ -/]*([@-~])/y
const NUMBERS_GROUP = 1
const ACTION_GROUP = 2
const FIRST_PRINTABLE = ' '
const DELETE = String.fromCharCode(DELETE_CODE)

/** No screen is this large; a position past it is a sequence gone wrong, and is not allowed to grow the text. */
const MOST_ROWS = 500
const MOST_COLUMNS = 1000

type Cursor = { row: number; column: number }

const within = (value: number, most: number) => Math.min(Math.max(value, 0), most - 1)

/**
 * The lines of text on a terminal screen the daemon sent. The daemon sends a screen as what a terminal would
 * need to draw it: colours, and a cursor jump wherever cells are blank, so neither its line breaks nor its
 * spaces are all in the text. This draws it and reads the text back; colours and modes are dropped.
 */
export function linesOfScreen(screen: string): string[] {
  const rows = new Map<number, string[]>()
  let cursor: Cursor = { row: 0, column: 0 }
  let saved = cursor

  // shortcut: a wide character takes one place here and two on the screen. The daemon moves along a line by
  // counts from where it is, and jumps to a place only on a new line, so the text after one still lines up.
  const write = (text: string, advance: boolean) => {
    const cells = rows.get(cursor.row) ?? []
    rows.set(cursor.row, cells)
    let column = cursor.column
    for (const character of text) {
      if (column >= MOST_COLUMNS) break
      while (cells.length < column) cells.push(' ')
      cells[column] = character
      column += 1
    }
    if (advance) cursor = { ...cursor, column }
  }

  const control = (numbers: readonly number[], action: string) => {
    const [first = 1, second = 1] = numbers
    if (action === 'H') cursor = { row: within(first - 1, MOST_ROWS), column: within(second - 1, MOST_COLUMNS) }
    else if (action === 'C') cursor = { ...cursor, column: within(cursor.column + first, MOST_COLUMNS) }
    else if (action === 'X') write(' '.repeat(within(first, MOST_COLUMNS)), false)
  }

  let at = 0
  while (at < screen.length) {
    const character = screen.charAt(at)
    if (character === ESC) {
      at = afterEscape(screen, at, {
        control,
        save: () => { saved = cursor },
        restore: () => { cursor = saved },
      })
    } else if (character === '\n') {
      // The daemon only ever sends a line feed after a carriage return, so starting the line here too loses
      // nothing, and a screen that is already plain text reads as its lines.
      cursor = { row: within(cursor.row + 1, MOST_ROWS), column: 0 }
      at += 1
    } else if (character === '\r') {
      cursor = { ...cursor, column: 0 }
      at += 1
    } else if (character === '\b') {
      cursor = { ...cursor, column: within(cursor.column - 1, MOST_COLUMNS) }
      at += 1
    } else if (character < FIRST_PRINTABLE || character === DELETE) {
      at += 1
    } else {
      const end = endOfText(screen, at)
      write(screen.slice(at, end), true)
      at = end
    }
  }

  const last = Math.max(-1, ...rows.keys())
  return Array.from({ length: last + 1 }, (_, row) => (rows.get(row) ?? []).join('').trimEnd())
}

type EscapeActions = {
  control: (numbers: readonly number[], action: string) => void
  save: () => void
  restore: () => void
}

/** Acts on the escape sequence that starts at `at`, and returns where the screen's text goes on after it. */
function afterEscape(screen: string, at: number, actions: EscapeActions): number {
  const next = screen.charAt(at + 1)
  if (next === '[') {
    CONTROL_SEQUENCE.lastIndex = at + 1
    const found = CONTROL_SEQUENCE.exec(screen)
    if (found === null) return screen.length
    actions.control((found[NUMBERS_GROUP] ?? '').split(';').map(countOf), found[ACTION_GROUP] ?? '')
    return CONTROL_SEQUENCE.lastIndex
  }
  if (next === ']') {
    const bell = screen.indexOf(BELL, at)
    const terminator = screen.indexOf(`${ESC}\\`, at)
    if (bell === -1 && terminator === -1) return screen.length
    return bell !== -1 && (terminator === -1 || bell < terminator) ? bell + 1 : terminator + `${ESC}\\`.length
  }
  if (next === '7') actions.save()
  if (next === '8') actions.restore()
  return at + `${ESC}${next}`.length
}

/** A number in a control sequence; one left out, or not a count, means 1. */
function countOf(part: string): number {
  const count = Number(part)
  return Number.isInteger(count) && count > 0 ? count : 1
}

function endOfText(screen: string, from: number): number {
  let end = from
  while (end < screen.length && screen.charAt(end) >= FIRST_PRINTABLE && screen.charAt(end) !== DELETE) end += 1
  return end
}
