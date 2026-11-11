import type { ToolEvent } from './chatEvents'
import { relativePath } from './stepInfo'

/**
 * What a tool step's row says: a plain verb and what it acted on. Read from the tool's own name and
 * input, so a fact the input does not hold (how many hits, a status code) is never shown.
 */
export type ToolFacts =
  | { readonly kind: 'file'; readonly verb: string; readonly path: string; readonly lines: string | null }
  | { readonly kind: 'folder'; readonly verb: string; readonly path: string }
  | { readonly kind: 'pattern'; readonly verb: string; readonly pattern: string; readonly where: string | null }
  | { readonly kind: 'address'; readonly verb: string; readonly address: string }
  | { readonly kind: 'query'; readonly verb: string; readonly query: string }
  /** A tool an extension (MCP server) provides. */
  | { readonly kind: 'extension'; readonly server: string; readonly tool: string; readonly target: string | null }
  | { readonly kind: 'other'; readonly verb: string; readonly target: string | null }

const EXTENSION_TOOL = /^mcp__(.+?)__(.+)$/
const PROTOCOL = /^https?:\/\//

type Input = Readonly<Record<string, unknown>>

/** The input as an object. It arrives as JSON text that the projection may have cut, and cut text is not JSON. */
function inputOf(event: ToolEvent): Input {
  if (event.input === null) return {}
  try {
    const parsed: unknown = JSON.parse(event.input)
    return typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed) ? (parsed as Input) : {}
  } catch (error) {
    if (error instanceof SyntaxError) return {}
    throw error
  }
}

const text = (input: Input, key: string): string | null => {
  const value = input[key]
  return typeof value === 'string' && value.trim() !== '' ? value.trim() : null
}

const count = (input: Input, key: string): number | null => {
  const value = input[key]
  return typeof value === 'number' && Number.isInteger(value) && value > 0 ? value : null
}

/** "lines 40–88" when the agent asked for a range; nothing when it read the whole file. */
function lineRange(input: Input): string | null {
  const first = count(input, 'offset')
  const length = count(input, 'limit')
  if (length === null) return first === null ? null : `from line ${String(first)}`
  const start = first ?? 1
  return `lines ${String(start)}–${String(start + length - 1)}`
}

export function toolFacts(event: ToolEvent, root: string | null): ToolFacts {
  const input = inputOf(event)
  const extension = EXTENSION_TOOL.exec(event.name)
  if (extension?.[1] !== undefined && extension[2] !== undefined) return { kind: 'extension', server: extension[1], tool: extension[2], target: event.target }
  const place = text(input, 'path')
  switch (event.name) {
    case 'Read': {
      const path = text(input, 'file_path') ?? event.target
      return path === null ? { kind: 'other', verb: 'Read', target: null } : { kind: 'file', verb: 'Read', path: relativePath(path, root), lines: lineRange(input) }
    }
    case 'Grep':
    case 'Glob': {
      const pattern = text(input, 'pattern') ?? event.target
      const verb = event.name === 'Grep' ? 'Search' : 'Find files'
      return pattern === null ? { kind: 'other', verb, target: null } : { kind: 'pattern', verb, pattern, where: place === null ? null : relativePath(place, root) }
    }
    case 'LS': {
      const path = place ?? event.target
      return path === null ? { kind: 'other', verb: 'List', target: null } : { kind: 'folder', verb: 'List', path: relativePath(path, root) }
    }
    case 'WebFetch': {
      const address = text(input, 'url') ?? event.target
      return address === null ? { kind: 'other', verb: 'Fetch', target: null } : { kind: 'address', verb: 'Fetch', address: address.replace(PROTOCOL, '') }
    }
    case 'WebSearch': {
      const query = text(input, 'query') ?? event.target
      return query === null ? { kind: 'other', verb: 'Web search', target: null } : { kind: 'query', verb: 'Web search', query }
    }
    default:
      return { kind: 'other', verb: event.name, target: event.target }
  }
}
