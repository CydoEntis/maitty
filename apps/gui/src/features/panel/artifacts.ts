import type { TouchedFile } from './panelModel'

/**
 * What counts as an artifact (v18 C.2): a document, report or picture the agent created, as opposed
 * to the source it edited. The daemon has no artifact concept, so this is a rule on the files the
 * history shows the agent creating, by extension. A source file the agent created stays a created
 * file in the Files list and is not an artifact.
 */

export type ArtifactKind = 'document' | 'report' | 'data' | 'image'

export type Artifact = {
  readonly file: string
  readonly kind: ArtifactKind
  readonly title: string
  readonly subtitle: string
}

const KIND_BY_EXTENSION: Readonly<Record<string, ArtifactKind>> = {
  md: 'document',
  mdx: 'document',
  txt: 'document',
  rst: 'document',
  html: 'report',
  htm: 'report',
  json: 'data',
  csv: 'data',
  xml: 'data',
  log: 'data',
  png: 'image',
  jpg: 'image',
  jpeg: 'image',
  gif: 'image',
  webp: 'image',
  svg: 'image',
}

const KIND_WORD: Record<ArtifactKind, string> = { document: 'Document', report: 'Report', data: 'Data file', image: 'Image' }

/** Markdown and plain text can be read in the panel; a picture cannot, because the daemon sends files as text. */
const PREVIEW_AS_TEXT: ReadonlySet<ArtifactKind> = new Set(['document', 'report', 'data'])

export function canPreview(kind: ArtifactKind): boolean {
  return PREVIEW_AS_TEXT.has(kind)
}

export function isMarkdown(file: string): boolean {
  return /\.mdx?$/i.test(file)
}

function extensionOf(file: string): string {
  const name = file.split(/[\\/]/).at(-1) ?? file
  const dot = name.lastIndexOf('.')
  return dot <= 0 ? '' : name.slice(dot + 1).toLowerCase()
}

/** The artifacts among the files the agent touched: only ones it created, newest touch last, as listed. */
export function artifactsOf(files: readonly TouchedFile[], displayPath: (file: string) => string): Artifact[] {
  return files.flatMap((touched): Artifact[] => {
    if (touched.kind !== 'created') return []
    const kind = KIND_BY_EXTENSION[extensionOf(touched.file)]
    if (kind === undefined) return []
    const path = displayPath(touched.file)
    const name = path.split(/[\\/]/).at(-1) ?? path
    const folder = path.slice(0, path.length - name.length).replace(/[\\/]$/, '')
    return [{ file: touched.file, kind, title: name, subtitle: folder === '' ? KIND_WORD[kind] : `${KIND_WORD[kind]} · ${folder}` }]
  })
}
