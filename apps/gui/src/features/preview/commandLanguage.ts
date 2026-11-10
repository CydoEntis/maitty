

const POWERSHELL_COMMAND = /powershell|pwsh/i

/** Which highlighting suits a command line, judged by the shell it runs. */
export function commandLanguage(command: string | undefined): string {
  return POWERSHELL_COMMAND.test(command ?? '') ? 'powershell' : 'sh'
}
