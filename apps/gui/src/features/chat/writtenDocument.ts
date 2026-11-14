import type { EditEvent } from './chatEvents'

/** A Markdown file the agent wrote, read from the lines its Write call carried. */
export type WrittenDocument = { readonly title: string | null; readonly excerpt: string; readonly lineCount: number; readonly text: string }

const MARKDOWN_FILE = /\.mdx?$/i
const HEADING = /^#{1,6}\s+(.*)$/
const EXCERPT_LINES = 4

/**
 * The document a step wrote, when it is one worth reading as a document: a finished Write of a
 * Markdown file whose whole text the history kept. Anything else is shown as a diff.
 */
export function documentOf(event: EditEvent): WrittenDocument | null {
  if (event.tool !== 'Write' || event.status !== 'completed' || event.file === null || !MARKDOWN_FILE.test(event.file)) return null
  if (event.diff.status !== 'available' || event.diff.truncated) return null
  const lines = event.diff.hunks.flatMap((hunk) => hunk.lines).filter((line) => line.kind === 'add').map((line) => line.text)
  if (lines.length === 0) return null
  const written = lines.filter((line) => line.trim() !== '')
  const title = written.map((line) => HEADING.exec(line)?.[1]).find((heading) => heading !== undefined) ?? null
  // Kept as lines, so a list stays a list when the excerpt is drawn as Markdown.
  const excerpt = written.filter((line) => !HEADING.test(line)).slice(0, EXCERPT_LINES).join('\n')
  return { title, excerpt, lineCount: lines.length, text: lines.join('\n') }
}
