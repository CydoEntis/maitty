import { toast } from 'sonner'
import { updateMachine } from '@/api/machines'
import { errorMessage } from '@/lib/error-message'
import { UPDATE_WORDS, type UpdateAction } from './model/machines'

// The update is the command-line app's, which leads each error it prints with its own name.
const CLI_PREFIX = /^maitty: /

function lines(text: string): { title: string; description?: string } {
  const [title = '', ...rest] = text.split('\n')
  const description = rest.join('\n').trim()
  return description === '' ? { title } : { title, description }
}

/**
 * Install or update the app on a machine with one toast that says it is running and then how it
 * ended: installed, already up to date, or the update's own reason it did not (refused, a newer
 * version there, ssh could not get in, a Windows machine). `onDone` runs after a success, so the
 * machine is dialled again on its new daemon.
 */
export async function runMachineUpdate(name: string, action: UpdateAction, whenIdle: boolean, onDone: () => void): Promise<void> {
  const words = UPDATE_WORDS[action]
  const id = toast.loading(words.running(name))
  try {
    const result = lines(await updateMachine(name, action === 'install', whenIdle))
    toast.success(result.title, { id, ...(result.description === undefined ? {} : { description: result.description }) })
    onDone()
  } catch (error) {
    toast.error(words.failed(name), { id, description: errorMessage(error, 'The update did not say why.').replace(CLI_PREFIX, '') })
  }
}
