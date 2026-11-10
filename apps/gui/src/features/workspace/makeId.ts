import type { MakeId } from './model/layout'

export const makeId: MakeId = (prefix) => `${prefix}-${crypto.randomUUID()}`
