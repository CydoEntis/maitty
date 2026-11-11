import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { toast } from '@/lib/toast'
import { copySelection } from './copySelection'

const writeText = vi.fn()

beforeEach(() => {
  writeText.mockReset().mockResolvedValue(undefined)
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } })
})

afterEach(() => {
  window.getSelection()?.removeAllRanges()
  cleanup()
})

function select(node: Node) {
  const range = document.createRange()
  range.selectNodeContents(node)
  const selection = window.getSelection()
  selection?.removeAllRanges()
  selection?.addRange(range)
}

function chat() {
  render(
    <>
      <div data-testid="chat" onMouseUp={copySelection}>
        <p>The lockfile was out of date.</p>
        <textarea aria-label="Message" defaultValue="draft" />
      </div>
      <p>Outside the chat.</p>
    </>,
  )
  return screen.getByTestId('chat')
}

describe('selecting text in the chat', () => {
  it('copies it when the mouse is released, and says so', async () => {
    const said = vi.spyOn(toast, 'success').mockImplementation(() => '')
    chat()
    select(screen.getByText('The lockfile was out of date.'))

    fireEvent.mouseUp(screen.getByText('The lockfile was out of date.'))

    expect(writeText).toHaveBeenCalledExactlyOnceWith('The lockfile was out of date.')
    await waitFor(() => { expect(said).toHaveBeenCalledWith('Copied', expect.anything()) })
  })

  it('says when the clipboard refused it', async () => {
    writeText.mockRejectedValue(new Error('denied'))
    const failed = vi.spyOn(toast, 'error').mockImplementation(() => '')
    chat()
    select(screen.getByText('The lockfile was out of date.'))

    fireEvent.mouseUp(screen.getByText('The lockfile was out of date.'))

    await waitFor(() => { expect(failed).toHaveBeenCalledWith('Could not copy the selection') })
  })

  it.each([
    ['a plain click', () => undefined, () => screen.getByText('The lockfile was out of date.')],
    ['a selection left outside the chat', () => { select(screen.getByText('Outside the chat.')) }, () => screen.getByText('The lockfile was out of date.')],
    ['a release in the message box', () => { select(screen.getByText('The lockfile was out of date.')) }, () => screen.getByLabelText('Message')],
  ])('copies nothing on %s', (_name, arrange, target) => {
    chat()
    arrange()

    fireEvent.mouseUp(target())

    expect(writeText).not.toHaveBeenCalled()
  })
})
