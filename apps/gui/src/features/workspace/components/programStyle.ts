import type { CSSProperties } from 'react'
import { programColor } from '../model/programColor'

/** The CSS variables that colour an unrecognised program's icon. */
export function programStyle(name: string): CSSProperties {
  const { from, to } = programColor(name)
  return { '--program-from': from, '--program-to': to } as CSSProperties
}
