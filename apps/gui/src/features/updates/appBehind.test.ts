import { describe, expect, it } from 'vitest'
import { appBehindStep } from './appBehind'

describe('app behind the service', () => {
  it('offers the download once a newer release is found', () => {
    expect(appBehindStep({ status: 'available', version: '0.70.0', notes: '' })).toEqual({ label: 'Download Maitty 0.70.0', action: 'download' })
  })

  it('offers the install once the download is ready', () => {
    expect(appBehindStep({ status: 'ready', version: '0.70.0' })).toEqual({ label: 'Install and restart', action: 'install' })
  })

  it('gives nothing to press while checking or downloading', () => {
    expect(appBehindStep({ status: 'checking' }).action).toBeNull()
    expect(appBehindStep({ status: 'downloading', version: '0.70.0', progress: 0.4 }).action).toBeNull()
  })

  it('lets a failed or empty check be run again', () => {
    expect(appBehindStep({ status: 'current' }).action).toBe('check')
    expect(appBehindStep({ status: 'failed', message: 'offline', during: 'check' }).action).toBe('check')
  })
})
