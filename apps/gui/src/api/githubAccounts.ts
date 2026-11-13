import { invoke } from '@tauri-apps/api/core'
import { z } from 'zod'
import { LOCAL_MACHINE } from './machines'
import { taggedAnswers } from './taggedAnswers'

const AccountSchema = z.object({ login: z.string(), active: z.boolean() })
const ReportSchema = z.object({
  installed: z.boolean(),
  accounts: z.array(AccountSchema),
  problem: z.string().nullable(),
})
const AnswerSchema = z.object({ machine: z.number().int(), tag: z.number().int(), report: ReportSchema })

/**
 * Who is signed in to GitHub on a machine, as its GitHub command line (`gh`) says: the active
 * account first. Never a token. `problem` is one line saying why the accounts could not be read.
 */
export type GitHubReport = z.infer<typeof ReportSchema>
export type GitHubAccount = z.infer<typeof AccountSchema>

/** How long a machine has to answer: its daemon gives `gh` fifteen seconds, and the answer has to cross. */
export const GITHUB_ANSWER_MS = 20_000

const ask = taggedAnswers('github-accounts', AnswerSchema)

/**
 * Starts GitHub's own sign-in (`gh auth login`) in a new terminal session on a machine (this
 * computer is machine 0). The person finishes it there and in their browser; nothing they type or
 * see there passes through here.
 */
export async function startGitHubSignIn(machine: number): Promise<void> {
  await invoke('start_github_sign_in', machine === LOCAL_MACHINE ? {} : { machine })
}

/**
 * Who is signed in to GitHub on a machine (this computer is machine 0). Nobody signed in, and no
 * `gh` there, are answers; an error is a machine that could not be asked or did not answer.
 */
export async function loadGitHubAccounts(machine: number): Promise<GitHubReport> {
  const send = (tag: number) => invoke('github_accounts', machine === LOCAL_MACHINE ? { tag } : { machine, tag })
  return (await ask(send, GITHUB_ANSWER_MS, 'The machine did not answer in time')).report
}
