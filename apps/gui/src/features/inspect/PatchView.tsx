type LineKind = 'meta' | 'hunk' | 'add' | 'remove' | 'context'

const META_PREFIXES = ['diff ', 'index ', '--- ', '+++ ', 'new file', 'deleted file', 'similarity', 'rename ', 'old mode', 'new mode', 'Binary files']

function classify(line: string): LineKind {
  if (line.startsWith('@@')) return 'hunk'
  if (META_PREFIXES.some((prefix) => line.startsWith(prefix))) return 'meta'
  if (line.startsWith('+')) return 'add'
  if (line.startsWith('-')) return 'remove'
  return 'context'
}

/** One file's unified diff, each line coloured by what it did. */
export function PatchView({ patch }: { patch: string }) {
  return (
    <pre className="mx-patch" aria-label="Diff">
      {patch.split(/\r?\n/).map((line, index) => (
        <span key={index} className={`mx-patch-line is-${classify(line)}`}>{line}{'\n'}</span>
      ))}
    </pre>
  )
}
