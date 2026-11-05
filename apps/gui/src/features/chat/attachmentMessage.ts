import type { AttachmentKind } from '@/api/attachments'

/**
 * How attachments travel inside a message (ADR 0053). The daemon's send is text, so the message
 * names each kept file by path in a block after the text; the agent reads the files with its own
 * tools. `buildAttachmentMessage` is the one place that makes the block, and `parseAttachmentMessage`
 * reads it back so a sent message shows chips, not paths. When the daemon takes a structured
 * attachment field, only these two change.
 */

export const ATTACHMENT_HEADER = 'Attached files (kept by Maitty; read them with your file tools):'

const LINE_PREFIX = '- '

/** Something the agent is told about by path. */
export type AttachedPath = { readonly path: string }

export function buildAttachmentMessage(text: string, attachments: readonly AttachedPath[]): string {
  if (attachments.length === 0) return text
  const lines = attachments.map((attachment) => `${LINE_PREFIX}${attachment.path}`)
  const block = [ATTACHMENT_HEADER, ...lines].join('\n')
  return text === '' ? block : `${text}\n\n${block}`
}

export type SentAttachment = { readonly path: string; readonly name: string; readonly kind: AttachmentKind }

/** Characters that are not text: control, format (zero-width, direction overrides) and separators. */
const NOT_TEXT = /[\p{C}\p{Zl}\p{Zp}]/gu

// A file kept on this computer is `attach-…`; one sent to another machine is `paste-…` (ADR 0050).
const KEPT_PREFIX = /^(?:attach|paste)-\d+-[0-9a-f]{8}-/
const PASTED = /^paste-\d+-[0-9a-f]{8}(?:-image)?(\.[A-Za-z0-9]+)$/
const IMAGE_EXTENSION = /\.(png|jpe?g|gif|webp)$/i
const ARCHIVE_EXTENSION = /\.(zip|tar|gz|tgz|7z|rar)$/i
const ABSOLUTE = /^(\/|[A-Za-z]:[\\/])/
/** Longest name a chip shows. */
const NAME_SHOWN = 80

function nameOf(path: string): string {
  const base = path.split(/[\\/]/).pop() ?? ''
  const pasted = PASTED.exec(base)
  const name = pasted ? `Pasted image${pasted[1] ?? ''}` : base.replace(KEPT_PREFIX, '')
  const shown = name.replace(NOT_TEXT, '')
  return (shown === '' ? 'file' : shown).slice(0, NAME_SHOWN)
}

export function kindOf(name: string): AttachmentKind {
  if (IMAGE_EXTENSION.test(name)) return 'image'
  return ARCHIVE_EXTENSION.test(name) ? 'archive' : 'file'
}

/**
 * Splits a sent message into its text and its attachments, or null when it carries none. Only a
 * trailing block of the exact shape counts, and only absolute paths: anything else a person typed
 * stays plain text.
 */
export function parseAttachmentMessage(message: string): { text: string; files: SentAttachment[] } | null {
  const at = message.lastIndexOf(ATTACHMENT_HEADER)
  if (at < 0 || (at > 0 && !message.slice(0, at).endsWith('\n\n'))) return null
  const lines = message.slice(at + ATTACHMENT_HEADER.length).split('\n')
  if (lines[0] !== '') return null
  const paths = lines.slice(1)
  if (paths.length === 0 || paths.some((line) => !line.startsWith(LINE_PREFIX) || !ABSOLUTE.test(line.slice(LINE_PREFIX.length)))) return null
  const files = paths.map((line) => {
    const path = line.slice(LINE_PREFIX.length)
    const name = nameOf(path)
    return { path, name, kind: kindOf(name) }
  })
  return { text: message.slice(0, at).replace(/\n+$/, ''), files }
}
