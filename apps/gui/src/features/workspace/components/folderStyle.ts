import type { CSSProperties } from 'react'
import { folderColor } from '../model/folderColor'

/** The CSS variables that colour a folder, so one stylesheet rule serves every group. */
export function folderStyle(key: string, chosen?: string): CSSProperties {
  const { from, to } = folderColor(key, chosen)
  return { '--folder-from': from, '--folder-to': to } as CSSProperties
}
