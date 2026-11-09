import { describe, expect, it, vi } from 'vitest'
import { ATTACHMENT_HEADER, buildAttachmentMessage, parseAttachmentMessage } from './attachmentMessage'
import { dropOnComposer, registerComposerDrop } from './composerDrop'

const KEPT = '/home/me/.cache/app/attachments/attach-1730000000000-0a1b2c3d-Design_Final.zip'
const PASTED = '/home/me/.cache/app/pastes/paste-1730000000001-0a1b2c3d.png'

describe('the message an attachment makes', () => {
  it('is the text alone when nothing is attached', () => {
    expect(buildAttachmentMessage('run it', [])).toBe('run it')
  })

  it('names each file by path after the text', () => {
    expect(buildAttachmentMessage('look at this', [{ path: KEPT }, { path: PASTED }])).toBe(`look at this\n\n${ATTACHMENT_HEADER}\n- ${KEPT}\n- ${PASTED}`)
  })

  it('is only the block when there is no text', () => {
    expect(buildAttachmentMessage('', [{ path: KEPT }])).toBe(`${ATTACHMENT_HEADER}\n- ${KEPT}`)
  })

  it('reads back as the text and chips with the names a person gave', () => {
    const message = buildAttachmentMessage('look at this', [{ path: KEPT }, { path: PASTED }])
    expect(parseAttachmentMessage(message)).toEqual({
      text: 'look at this',
      files: [
        { path: KEPT, name: 'Design_Final.zip', kind: 'archive' },
        { path: PASTED, name: 'Pasted image.png', kind: 'image' },
      ],
    })
    expect(parseAttachmentMessage(buildAttachmentMessage('', [{ path: KEPT }]))?.text).toBe('')
  })

  it('still reads back when the history adds trailing blank lines or CRLF', () => {
    const message = buildAttachmentMessage('look', [{ path: KEPT }])
    expect(parseAttachmentMessage(`${message}\n\n`)?.files).toHaveLength(1)
    expect(parseAttachmentMessage(message.replace(/\n/g, '\r\n'))?.text).toBe('look')
  })

  it('leaves a message alone that only looks like one', () => {
    expect(parseAttachmentMessage('plain')).toBeNull()
    expect(parseAttachmentMessage(ATTACHMENT_HEADER)).toBeNull()
    // Text typed after the header, a relative path, or a mix is plain text.
    expect(parseAttachmentMessage(`${ATTACHMENT_HEADER}\n- ${KEPT}\nand also this`)).toBeNull()
    expect(parseAttachmentMessage(`${ATTACHMENT_HEADER}\n- ../../etc/passwd`)).toBeNull()
    expect(parseAttachmentMessage(`hi ${ATTACHMENT_HEADER}\n- ${KEPT}`)).toBeNull()
  })

  it('shows a hostile name as plain text: no control or direction characters', () => {
    const path = '/c/attach-1730000000000-0a1b2c3d-a\u202egnp\u0007.exe'
    const parsed = parseAttachmentMessage(`${ATTACHMENT_HEADER}\n- ${path}`)
    expect(parsed?.files[0]?.name).toBe('agnp.exe')
  })

  it('reads a Windows path', () => {
    const path = 'C:\\Users\\me\\AppData\\Local\\app\\attachments\\attach-1730000000000-0a1b2c3d-notes.txt'
    expect(parseAttachmentMessage(`${ATTACHMENT_HEADER}\n- ${path}`)?.files).toEqual([{ path, name: 'notes.txt', kind: 'file' }])
  })
})

describe('a file kept on another machine', () => {
  it('reads back with the name a person gave, not the prefix the far side added (ADR 0007)', () => {
    const log = '/home/dev/.cache/pastes/paste-1730000000002-0a1b2c3d-build_log.txt'
    const shot = '/home/dev/.cache/pastes/paste-1730000000003-0a1b2c3d-image.jpg'
    const message = buildAttachmentMessage('', [{ path: log }, { path: shot }])
    expect(parseAttachmentMessage(message)?.files).toEqual([
      { path: log, name: 'build_log.txt', kind: 'file' },
      { path: shot, name: 'Pasted image.jpg', kind: 'image' },
    ])
  })
})

describe('a drop on a composer', () => {
  it('goes to the composer that holds the element under the pointer, and to no other', () => {
    const inside = document.createElement('div')
    const child = document.createElement('span')
    inside.append(child)
    const elsewhere = document.createElement('div')
    const onFiles = vi.fn()
    const stop = registerComposerDrop(inside, onFiles)

    expect(dropOnComposer(['/a.png'], child)).toBe(true)
    expect(onFiles).toHaveBeenCalledWith(['/a.png'])
    expect(dropOnComposer(['/b.png'], elsewhere)).toBe(false)
    expect(dropOnComposer(['/b.png'], null)).toBe(false)
    expect(onFiles).toHaveBeenCalledTimes(1)

    stop()
    expect(dropOnComposer(['/c.png'], child)).toBe(false)
  })
})
