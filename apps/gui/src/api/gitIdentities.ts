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
