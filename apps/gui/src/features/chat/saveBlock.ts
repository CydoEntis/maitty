import { saveExport } from '@/api/exports'
import { readSettingsPreferences } from '@/features/settings/preferences'
import { errorMessage } from '@/lib/error-message'
import { toast } from '@/lib/toast'
import { exportName } from './exportBlock'

/** Saves a block of a reply into the export folder chosen in Settings, and says where it went or why it did not. */
export function saveBlock(what: string, extension: string, text: string): void {
  // Read when the person saves, so a folder just chosen in Settings is the one used.
  const folder = readSettingsPreferences().exportFolder
  saveExport(folder, exportName(what, extension, new Date()), text).then(
    (path) => { toast.success(`Saved to ${path}`) },
    (error: unknown) => { toast.error(errorMessage(error, 'Could not save the file')) },
  )
}

export function copyBlock(text: string, failed: string): void {
  navigator.clipboard.writeText(text).then(
    () => { toast.message('Copied') },
    () => { toast.error(failed) },
  )
}
