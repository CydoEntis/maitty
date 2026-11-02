import { HighlightedLine } from '@/components/HighlightedLine'

const LINE_BREAK = /\r?\n/
const NEWLINE = '\n'
// The daemon heads each file's diff with "path (update)".
const FILE_HEADER = /^(.+) \((?:add|update|delete|move|[a-z]+)\)$/

type DiffLineKind = 'header' | 'hunk' | 'add' | 'remove' | 'context'

function classify(line: string, previous: string | undefined): DiffLineKind {
  if (line.startsWith('@@')) return 'hunk'
  if (line.startsWith('+') && !line.startsWith('+++')) return 'add'
  if (line.startsWith('-') && !line.startsWith('---')) return 'remove'
  if (FILE_HEADER.test(line) && (previous === undefined || previous === ''))
    return 'header'
  return 'context'
}

/** A file edit as the agent made it: each file's name, then its colored diff. */
export function DiffText({ text }: { text: string }) {
  const lines = text.split(LINE_BREAK)
  // Each line's file comes from the header above it, worked out up front so rendering stays pure.
  const rows = lines.reduce<{ kind: DiffLineKind; path: string }[]>(
    (all, line, index) => {
      const kind = classify(line, lines[index - 1])
      const path =
        kind === 'header'
          ? (FILE_HEADER.exec(line)?.[1] ?? '')
          : (all[index - 1]?.path ?? '')
      return [...all, { kind, path }]
    },
    [],
  )
  return (
    <pre className="diff-text">
      {lines.map((line, index) => {
        const { kind, path } = rows[index] ?? { kind: 'context', path: '' }
        const changed = kind === 'add' || kind === 'remove'
        return (
          <span key={index} className={`diff-text-line is-${kind}`}>
            {changed ? (
              <>
                <i aria-hidden="true">{kind === 'add' ? '+' : '−'}</i>
                <HighlightedLine line={line.slice(1)} path={path} />
              </>
            ) : (
              line
            )}
            {NEWLINE}
          </span>
        )
      })}
    </pre>
  )
}
