/**
 * A line comment, as the message the agent receives and the chat shows
 * (v18 C.1 "comment bubble", C.2 line comments). The daemon's send takes text only, so the comment
 * travels as one message that holds all three fields of `{file, line, text}`, in a shape the chat can
 * read back to draw a bubble. Only a message this app composed can match: nothing is parsed from
 * the agent's output.
 */

export type LineComment = {
  readonly file: string
  /** The line number in the file as it is now (the diff's new side). */
  readonly line: number
  /** The line's code, so the agent sees what the comment is about even if the file moved on. */
  readonly code: string
  readonly text: string
}

const HEADER = 'Comment on '
const CODE_PREFIX = '> '
// "Comment on <file>:<line>", the quoted line, a blank line, then the comment.
const PATTERN = /^Comment on (.+):(\d+)\n> (.*)\n\n([\s\S]+)$/

/** The one-line limit of the quoted code, so a minified line cannot flood the message. */
const CODE_LIMIT = 240

export function formatComment(comment: LineComment): string {
  const code = comment.code.replace(/[\r\n]+/g, ' ').slice(0, CODE_LIMIT)
  return `${HEADER}${comment.file}:${String(comment.line)}\n${CODE_PREFIX}${code}\n\n${comment.text.trim()}`
}

/** The comment a message holds, or `null` for any other message. */
export function parseComment(message: string): LineComment | null {
  const match = PATTERN.exec(message)
  if (match === null) return null
  const [, file, line, code, text] = match
  if (file === undefined || line === undefined || code === undefined || text === undefined) return null
  return { file, line: Number(line), code, text }
}
