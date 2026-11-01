export type TokenKind = 'plain' | 'comment' | 'string' | 'number' | 'keyword'
export type Token = { text: string; kind: TokenKind }

const SLASH_LANGUAGES = new Set([
  'ts',
  'tsx',
  'js',
  'jsx',
  'mjs',
  'cjs',
  'rs',
  'go',
  'java',
  'kt',
  'swift',
  'c',
  'h',
  'cpp',
  'hpp',
  'cs',
  'css',
  'scss',
  'json',
])
const HASH_LANGUAGES = new Set([
  'py',
  'sh',
  'bash',
  'zsh',
  'rb',
  'toml',
  'yaml',
  'yml',
])

// Common to the languages above; enough to make code scannable, not a parser.
const KEYWORDS = new Set([
  'abstract',
  'as',
  'async',
  'await',
  'break',
  'case',
  'catch',
  'class',
  'const',
  'continue',
  'def',
  'default',
  'else',
  'enum',
  'export',
  'extends',
  'false',
  'finally',
  'fn',
  'for',
  'from',
  'func',
  'function',
  'if',
  'impl',
  'import',
  'in',
  'interface',
  'let',
  'match',
  'mod',
  'mut',
  'new',
  'null',
  'None',
  'of',
  'pub',
  'return',
  'self',
  'static',
  'struct',
  'super',
  'switch',
  'this',
  'throw',
  'trait',
  'true',
  'True',
  'False',
  'try',
  'type',
  'use',
  'var',
  'while',
  'with',
  'yield',
])

// Skip very long lines (minified bundles) so a preview never stalls the UI.
const MAX_HIGHLIGHT_LINE_LENGTH = 1000

const STRING =
  String.raw`"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|` + '`(?:[^`\\\\]|\\\\.)*`'
const SLASH_PATTERN = new RegExp(
  String.raw`(\/\/.*|\/\*.*?\*\/)|(${STRING})|(\b\d[\d_.]*\b)|(\b[A-Za-z_][A-Za-z0-9_]*\b)`,
  'g',
)
const HASH_PATTERN = new RegExp(
  String.raw`(#.*)|(${STRING})|(\b\d[\d_.]*\b)|(\b[A-Za-z_][A-Za-z0-9_]*\b)`,
  'g',
)

function extensionOf(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? ''
  const dot = name.lastIndexOf('.')
  return dot < 0 ? '' : name.slice(dot + 1).toLowerCase()
}

/** Split one line into tokens for its file type; unknown types stay plain. */
export function highlightLine(line: string, path: string): Token[] {
  const extension = extensionOf(path)
  const pattern = SLASH_LANGUAGES.has(extension)
    ? SLASH_PATTERN
    : HASH_LANGUAGES.has(extension)
      ? HASH_PATTERN
      : null
  if (!pattern || line.length > MAX_HIGHLIGHT_LINE_LENGTH)
    return [{ text: line, kind: 'plain' }]

  const tokens: Token[] = []
  let cursor = 0
  pattern.lastIndex = 0
  for (const match of line.matchAll(pattern)) {
    const [text, comment, string, number, word] = match
    const start = match.index
    if (start > cursor)
      tokens.push({ text: line.slice(cursor, start), kind: 'plain' })
    const kind: TokenKind = comment
      ? 'comment'
      : string
        ? 'string'
        : number
          ? 'number'
          : word && KEYWORDS.has(word)
            ? 'keyword'
            : 'plain'
    tokens.push({ text, kind })
    cursor = start + text.length
  }
  if (cursor < line.length)
    tokens.push({ text: line.slice(cursor), kind: 'plain' })
  return tokens
}
