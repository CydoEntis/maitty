import { invoke } from '@tauri-apps/api/core'
import { z } from 'zod'
import { LOCAL_MACHINE } from './machines'

const IdentitySchema = z.object({ name: z.string(), email: z.string(), login: z.string() })
const RuleSchema = z.object({ folder: z.string(), identity: IdentitySchema.nullable() })
const ReportSchema = z.object({
  installed: z.boolean(),
  name: z.string().nullable(),
  email: z.string().nullable(),
  rules: z.array(RuleSchema),
  problem: z.string().nullable(),
})

/** Who commits, and as which GitHub account git signs in, inside a folder with a rule Maitty wrote. */
export type GitIdentity = z.infer<typeof IdentitySchema>
/** A folder whose checkouts use another identity. `identity` is null for a rule set outside Maitty: only its folder is known. */
export type GitIdentityRule = z.infer<typeof RuleSchema>
/**
 * What a machine's global git config says: the name and email commits carry by default (each null
 * when not set) and its folder rules. Never a credential. `problem` is one line saying why the
 * config could not be read.
 */
export type GitIdentityReport = z.infer<typeof ReportSchema>

/**
 * Which commit identity git uses on a machine (this computer is machine 0), and for which folders
 * a different one applies. Nothing is written there. No git on the machine is an answer; an error
 * is a machine that could not be asked or did not answer.
 */
export async function loadGitIdentities(machine: number): Promise<GitIdentityReport> {
  return ReportSchema.parse(await invoke<unknown>('git_identities', machine === LOCAL_MACHINE ? {} : { machine }))
}

const WriteSchema = z.object({ file: z.string(), lines: z.array(z.string()) })

/** The lines of one git config file, each `key = value`, that a change to a folder's rule writes, would write or took away. */
export type GitConfigWrite = z.infer<typeof WriteSchema>

/** What to do to a folder's rule: see the lines first, write them, or take the rule away. */
export type GitRuleChange =
  | { kind: 'preview'; folder: string; identity: GitIdentity }
  | { kind: 'apply'; folder: string; identity: GitIdentity }
  | { kind: 'remove'; folder: string }

/** `change` as the daemon reads it. */
function wire(change: GitRuleChange): unknown {
  if (change.kind === 'remove') return { Remove: { folder: change.folder } }
  const body = { folder: change.folder, identity: change.identity }
  return change.kind === 'preview' ? { Preview: body } : { Apply: body }
}

/**
 * Previews, writes or removes a folder's GitHub account and commit identity rule on a machine (this
 * computer is machine 0). Resolves to the lines by file; rejects with the machine's reason, and then
 * nothing was written.
 */
export async function changeGitRule(machine: number, change: GitRuleChange): Promise<GitConfigWrite[]> {
  const where = machine === LOCAL_MACHINE ? {} : { machine }
  return z.array(WriteSchema).parse(await invoke<unknown>('git_identity_rule', { ...where, change: wire(change) }))
}

/** `path` with `/` between its parts and no closing slash. */
function plain(path: string): string {
  return path.replace(/\\/g, '/').replace(/\/+$/, '')
}

/**
 * The rule Maitty wrote that covers `folder`: the deepest one whose folder is it or holds it. A
 * Windows folder is matched without regard to case, as git matches it there. Null when none does.
 */
export function ruleCovering(rules: readonly GitIdentityRule[], folder: string): GitIdentity | null {
  const target = plain(folder)
  const windows = /^[A-Za-z]:\//.test(target)
  const same = (a: string, b: string) => (windows ? a.toLowerCase() === b.toLowerCase() : a === b)
  const holds = (rule: GitIdentityRule) => {
    const base = plain(rule.folder)
    return same(target, base) || same(target.slice(0, base.length + 1), `${base}/`)
  }
  const covering = rules.filter((rule) => rule.identity !== null && holds(rule))
  const deepest = covering.sort((a, b) => b.folder.length - a.folder.length)[0]
  return deepest?.identity ?? null
}
