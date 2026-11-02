import type { SessionStatus } from './session'

export const STATUS_LABEL: Record<SessionStatus, string> = {
  working: 'Working',
  needs: 'Needs you',
  done: 'Done',
  error: 'Exited',
  idle: 'Idle',
  unknown: 'Unknown',
}

export const STATUS_HELP: Record<SessionStatus, string> = {
  working: 'A program is running in this session, such as an agent or a command. It is busy, so you can leave it.',
  needs: 'The agent is waiting for you, for example to approve something or answer a question. Answer it in the terminal.',
  done: 'The last job finished while you were away. This clears when you run your next command.',
  error: 'The last command ended with an error. The output above shows what went wrong.',
  idle: 'Nothing is running. The shell is waiting for your next command.',
  unknown: 'maittyd cannot be reached, so this is the last state seen. It returns to normal when the connection does.',
}
