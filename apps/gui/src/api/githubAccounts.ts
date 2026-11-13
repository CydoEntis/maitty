import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { z } from 'zod'
import { LOCAL_MACHINE } from './machines'

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

const waiting = new Map<number, (report: GitHubReport) => void>()
let nextTag = 1
let listening: Promise<void> | null = null

function listenForAnswers(): Promise<void> {
  listening ??= listen<unknown>('github-accounts', (event) => {
    const answer = AnswerSchema.safeParse(event.payload)
    if (!answer.success) return
    const resolve = waiting.get(answer.data.tag)
    waiting.delete(answer.data.tag)
    resolve?.(answer.data.report)
  }).then(() => undefined)
  listening.catch(() => { listening = null })
  return listening
}

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
  await listenForAnswers()
  const tag = nextTag
  nextTag += 1
  return new Promise<GitHubReport>((resolve, reject) => {
    const timer = window.setTimeout(() => {
      waiting.delete(tag)
      reject(new Error('The machine did not answer in time'))
    }, GITHUB_ANSWER_MS)
    waiting.set(tag, (report) => {
      window.clearTimeout(timer)
      resolve(report)
    })
    const args = machine === LOCAL_MACHINE ? { tag } : { machine, tag }
    invoke('github_accounts', args).catch((error: unknown) => {
      window.clearTimeout(timer)
      waiting.delete(tag)
      reject(error instanceof Error ? error : new Error(String(error)))
    })
  })
}
