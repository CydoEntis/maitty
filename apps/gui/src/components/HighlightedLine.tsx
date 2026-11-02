import { highlightLine } from '@/lib/highlight'

export function HighlightedLine({
  line,
  path,
}: {
  line: string
  path: string
}) {
  return (
    <>
      {highlightLine(line, path).map((token, index) =>
        token.kind === 'plain' ? (
          token.text
        ) : (
          <span key={index} className={`tok-${token.kind}`}>
            {token.text}
          </span>
        ),
      )}
    </>
  )
}
