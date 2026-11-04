import { describe, expect, it } from 'vitest'
import capabilityText from '../../../src-tauri/capabilities/default.json?raw'

// updater.ts downloads in the background and installs on restart: two Tauri commands. The
// combined `updater:allow-download-and-install` permission covers neither, and a missing
// permission only fails at runtime, on the user's machine ("updater.download not allowed").
const { permissions } = JSON.parse(capabilityText) as { permissions: string[] }

describe('the window capability for updates', () => {
  it.each([
    'updater:allow-check',
    'updater:allow-download',
    'updater:allow-install',
    'process:allow-restart',
  ])('allows %s', (permission) => {
    expect(permissions).toContain(permission)
  })
})
