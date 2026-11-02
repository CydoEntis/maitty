import { HighlightedLine } from '@/components/HighlightedLine'
import { extensionForLanguage } from '@/lib/highlight'

const LINE_BREAK = /\r?\n/

/**
 * Colored code for a block: pass the file path, or the language name from a
 * code fence. Lines keep their own breaks so the text stays exactly as given.
 */
export function HighlightedCode({
  code,
  path,
  language,
}: {
  code: string
  path?: string
  language?: string
}) {
  const hint = path ?? `code.${extensionForLanguage(language ?? '')}`
  return (
    <>
      {code.split(LINE_BREAK).map((line, index, all) => (
        <span key={index} className="code-line">
          <HighlightedLine line={line} path={hint} />
          {index < all.length - 1 ? '\n' : ''}
        </span>
      ))}
    </>
  )
}
