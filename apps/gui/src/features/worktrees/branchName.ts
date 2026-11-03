// Characters git refuses anywhere in a branch name; control characters are checked separately.
const FORBIDDEN = /[\s~^:?*[\\]/
const FIRST_PRINTABLE = 0x20
const DELETE = 0x7f

const hasControlCharacter = (name: string): boolean =>
  Array.from({ length: name.length }, (_, index) => name.charCodeAt(index)).some((code) => code < FIRST_PRINTABLE || code === DELETE)

/** Why a branch name will not do, or null when it will. The daemon and git check it again. */
export function branchNameProblem(name: string): string | null {
  if (name === '') return 'Name the branch'
  if (name.startsWith('-')) return 'A branch name cannot start with a dash'
  if (name.startsWith('/') || name.endsWith('/')) return 'A branch name cannot start or end with a slash'
  if (name.endsWith('.') || name.endsWith('.lock')) return 'A branch name cannot end with a dot or .lock'
  if (name.includes('..') || name.includes('//') || name.includes('@{')) return 'A branch name cannot hold .. or // or @{'
  if (FORBIDDEN.test(name) || hasControlCharacter(name)) return 'A branch name cannot hold spaces or ~ ^ : ? * [ \\'
  return null
}
