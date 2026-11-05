import { renderHook } from '@testing-library/react'
import { toast } from 'sonner'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { forgetComposers, peekComposer } from './composerStore'
import { useAttachmentIntake } from './useAttachmentIntake'

const attach = vi.hoisted(() => ({ attachLocalFile: vi.fn(), chooseFilesToAttach: vi.fn() }))

vi.mock('@/api/attachments', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/attachments')>()), ...attach }))

beforeEach(() => {
  forgetComposers()
  vi.restoreAllMocks()
  attach.attachLocalFile.mockReset()
  attach.chooseFilesToAttach.mockReset()
})

describe('attaching in a chat on another machine', () => {
  it('says it is not available yet and keeps nothing, because only this computer is built (ADR 0053)', () => {
    const error = vi.spyOn(toast, 'error')
    const { result } = renderHook(() => useAttachmentIntake({ conversation: 7, blocked: null, machine: 'build-box' }))

    result.current.addPaths(['/home/me/shot.png'])
    result.current.pick()

    expect(error).toHaveBeenCalledWith('Attaching files to a chat on build-box is not available yet')
    expect(attach.attachLocalFile).not.toHaveBeenCalled()
    expect(attach.chooseFilesToAttach).not.toHaveBeenCalled()
    expect(peekComposer(7).attachments).toEqual([])
  })
})
