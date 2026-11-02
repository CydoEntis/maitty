export type TokenKind =
  | 'plain'
  | 'comment'
  | 'string'
  | 'number'
  | 'keyword'
  | 'variable'
  | 'flag'
export type Token = { text: string; kind: TokenKind }

const SLASH_LANGUAGES = new Set([
  'ts', 'tsx', 'js', 'jsx', 'mjs', 'cjs', 'rs', 'go', 'java', 'kt', 'swift',
  'c', 'h', 'cpp', 'hpp', 'cs', 'css', 'scss', 'json',
])
// Shell-like languages: # comments, $variables and -flags are worth coloring.
const HASH_LANGUAGES = new Set([
  'py', 'sh', 'bash', 'zsh', 'rb', 'toml', 'yaml', 'yml', 'ps1',
])
const SHELL_LANGUAGES = new Set(['sh', 'bash', 'zsh', 'ps1'])

// Common to the languages above; enough to make code scannable, not a parser.
const KEYWORDS = new Set([
  'abstract', 'as', 'async', 'await', 'break', 'case', 'catch', 'chan', 'class',
  'const', 'continue', 'def', 'default', 'defer', 'do', 'done', 'elif', 'else',
  'enum', 'esac', 'export', 'extends', 'false', 'fi', 'finally', 'fn', 'for',
  'foreach', 'from', 'func', 'function', 'go', 'if', 'impl', 'import', 'in',
  'interface', 'let', 'map', 'match', 'mod', 'mut', 'new', 'nil', 'null',
  'None', 'of', 'package', 'param', 'pub', 'range', 'return', 'select', 'self',
  'static', 'struct', 'super', 'switch', 'then', 'this', 'throw', 'trait',
  'true', 'True', 'False', 'try', 'type', 'use', 'var', 'while', 'with', 'yield',
])

// Skip very long lines (minified bundles) so a preview never stalls the UI.
const MAX_HIGHLIGHT_LINE_LENGTH = 1000

const STRING =
  String.raw`"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|` + '`(?:[^`\\\\]|\\\\.)*`'
const WORD = String.raw`(\b[A-Za-z_][A-Za-z0-9_]*\b)`
const NUMBER = String.raw`(\b\d[\d_.]*\b)`
// Groups: comment, string, number, word, variable, flag (the last two only in shells).
const SLASH_PATTERN = new RegExp(
  String.raw`(\/\/.*|\/\*.*?\*\/)|(${STRING})|${NUMBER}|${WORD}`,
  'g',
)
const HASH_PATTERN = new RegExp(
  String.raw`(#.*)|(${STRING})|${NUMBER}|${WORD}`,
  'g',
)
const SHELL_PATTERN = new RegExp(
  String.raw`(#.*)|(${STRING})|${NUMBER}|${WORD}|(\$[A-Za-z_][\w:]*|\$\{[^}]*\})|((?<![\w-])--?[A-Za-z][\w-]*)`,
  'g',
)

const LANGUAGE_EXTENSIONS: Record<string, string> = {
  typescript: 'ts', ts: 'ts', tsx: 'tsx', javascript: 'js', js: 'js', jsx: 'jsx',
  node: 'js', go: 'go', golang: 'go', rust: 'rs', rs: 'rs', python: 'py',
  py: 'py', ruby: 'rb', java: 'java', kotlin: 'kt', swift: 'swift', c: 'c',
  cpp: 'cpp', 'c++': 'cpp', csharp: 'cs', cs: 'cs', css: 'css', scss: 'scss',
  json: 'json', yaml: 'yml', yml: 'yml', toml: 'toml', bash: 'sh', sh: 'sh',
  shell: 'sh', zsh: 'zsh', console: 'sh', powershell: 'ps1', pwsh: 'ps1',
  ps1: 'ps1', ps: 'ps1',
}

/** A file extension for a code fence's language name ("golang" is "go"); empty if unknown. */
export function extensionForLanguage(language: string): string {
  return LANGUAGE_EXTENSIONS[language.trim().toLowerCase()] ?? ''
}

function extensionOf(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? ''
  const dot = name.lastIndexOf('.')
  return dot < 0 ? '' : name.slice(dot + 1).toLowerCase()
}

/** Split one line into tokens for its file type; unknown types stay plain. */
export function highlightLine(line: string, path: string): Token[] {
  const extension = extensionOf(path)
  const pattern = SHELL_LANGUAGES.has(extension)
    ? SHELL_PATTERN
    : SLASH_LANGUAGES.has(extension)
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
    const [text, comment, string, number, word, variable, flag] = match
    const start = match.index
    if (start > cursor)
      tokens.push({ text: line.slice(cursor, start), kind: 'plain' })
    const kind: TokenKind = comment
      ? 'comment'
      : string
        ? 'string'
        : number
          ? 'number'
          : variable
            ? 'variable'
            : flag
              ? 'flag'
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
