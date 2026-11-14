import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it } from 'vitest'
import { previewMessages, type Message } from '@/data/preview'
import App from './App'

describe('conversation workspace', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('opens a selected session and shows its current status', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.click(screen.getByRole('button', { name: /^Add a conversation reconnect test, Working, maitty$/i }))

    expect(screen.getByRole('heading', { name: 'Add a conversation reconnect test' })).toBeInTheDocument()
    expect(screen.getByRole('status', { name: 'Current status: Working' })).toBeInTheDocument()
    expect(document.querySelector('.app-shell > [role="status"][aria-live="polite"]')).toHaveAttribute('aria-atomic', 'true')
    expect(screen.getByRole('status', { name: 'Local preview · daemon not connected' })).toBeInTheDocument()
  })

  it('keeps Changes and Files closed until the user opens a panel', async () => {
    const user = userEvent.setup()
    renderApp()

    expect(screen.queryByRole('complementary', { name: 'Conversation details' })).not.toBeInTheDocument()
    await user.click(await findReviewButton())
    await user.click(screen.getByRole('tab', { name: 'Files' }))

    expect(screen.getByRole('tab', { name: 'Files' })).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByText('Files need the desktop app')).toBeInTheDocument()
  })

  it('keeps opened conversations in tabs without closing daemon sessions', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.click(screen.getByRole('button', { name: /^Add a conversation reconnect test, Working, maitty$/i }))
    expect(screen.getByRole('tab', { name: /Add a conversation reconnect test/ })).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByRole('tab', { name: /Tighten the branch view navigation/ })).toBeInTheDocument()

    await user.keyboard('{Control>}{Tab}{/Control}')
    expect(screen.getByRole('tab', { name: /Tighten the branch view navigation/ })).toHaveAttribute('aria-selected', 'true')

    await user.click(screen.getByRole('button', { name: 'Close Add a conversation reconnect test tab' }))
    expect(screen.queryByRole('tab', { name: /Add a conversation reconnect test/ })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: /^Add a conversation reconnect test, Working, maitty$/i })).toBeInTheDocument()
  })

  it('labels Claude replies as Claude in the conversation timeline', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.click(
      screen.getByRole('button', {
        name: /^Review the daemon restore flow, Done, maitty$/i,
      }),
    )

    expect(screen.getAllByText(/Claude · /).length).toBeGreaterThan(0)
    expect(screen.queryByText('GPT-5 Codex')).not.toBeInTheDocument()
  })

  it('lets keyboard users skip the conversation list and focus the main conversation', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.tab()
    expect(screen.getByRole('link', { name: 'Skip to conversation' })).toHaveFocus()
    await user.keyboard('{Enter}')

    expect(screen.getByRole('main')).toHaveFocus()
  })

  it('finds and opens a thread from the keyboard switcher', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.keyboard('{Control>}k{/Control}')
    const dialog = screen.getByRole('dialog')
    await user.type(within(dialog).getByRole('textbox', { name: 'Search actions and sessions' }), 'conversation reconnect')
    await user.click(within(dialog).getByRole('button', { name: /add a conversation reconnect test/i }))

    expect(screen.getByRole('heading', { name: 'Add a conversation reconnect test' })).toBeInTheDocument()
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('opens the new conversation dialog with Ctrl+N', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.keyboard('{Control>}n{/Control}')

    expect(screen.getByRole('dialog')).toHaveTextContent('New conversation')
  })

  it('starts a session in a folder with Ctrl+O', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.keyboard('{Control>}o{/Control}')

    expect(screen.getByRole('dialog')).toHaveTextContent('New conversation')
  })

  it('opens the shortcut map with question mark', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.keyboard('?')

    expect(screen.getByRole('dialog')).toHaveTextContent('Keyboard shortcuts')
    expect(screen.getByRole('dialog')).toHaveTextContent('Start a session in a folder')
    expect(screen.getByRole('dialog')).toHaveTextContent('Open settings')
    expect(screen.getByRole('dialog')).toHaveTextContent('Switch between chat and terminal')
    expect(screen.getByRole('dialog')).toHaveTextContent('Move through switcher results')
    expect(screen.getByRole('dialog')).toHaveTextContent('Open the focused switcher result')
    expect(screen.getByRole('dialog')).toHaveTextContent('Close the command switcher')
  })

  it('opens the focused switcher conversation in the terminal with Ctrl+Enter', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.keyboard('{Control>}k{/Control}')
    await user.type(screen.getByRole('textbox', { name: 'Search actions and sessions' }), 'Check Windows named pipe errors')
    await user.keyboard('{ArrowDown}')
    await user.keyboard('{Control>}{Enter}{/Control}')

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(screen.getByText(/Preview terminal output/)).toBeInTheDocument()
  })

  it('collapses and restores the desktop sidebar with Ctrl+backslash', async () => {
    const user = userEvent.setup()
    renderApp()
    const sidebar = screen.getByRole('complementary', { name: 'Conversations' })

    fireEvent.keyDown(window, { key: '\\', ctrlKey: true })

    expect(sidebar).toHaveClass('is-desktop-collapsed')
    await user.click(screen.getByRole('button', { name: 'Expand conversation sidebar' }))
    expect(sidebar).not.toHaveClass('is-desktop-collapsed')
  })

  it('opens and closes the compact sidebar drawer with Ctrl+backslash', () => {
    const previousWidth = window.innerWidth
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 800 })
    const app = renderApp()

    try {
      const sidebar = screen.getByRole('complementary', { name: 'Conversations' })
      expect(sidebar).toHaveClass('is-compact')
      fireEvent.keyDown(window, { key: '\\', ctrlKey: true })
      expect(sidebar).toHaveClass('is-drawer-open')
      fireEvent.keyDown(window, { key: '\\', ctrlKey: true })
      expect(sidebar).not.toHaveClass('is-drawer-open')
    } finally {
      app.unmount()
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: previousWidth })
    }
  })

  it('moves between context panel tabs with arrow keys', async () => {
    const user = userEvent.setup()
    renderApp()
    await userEvent.setup().click(
      await findReviewButton(),
    )

    const changesTab = screen.getByRole('tab', { name: /^Changes/ })
    const filesTab = screen.getByRole('tab', { name: 'Files' })
    const contextTab = screen.getByRole('tab', { name: 'Context' })
    changesTab.focus()
    await user.keyboard('{ArrowRight}')
    expect(filesTab).toHaveAttribute('aria-selected', 'true')
    expect(filesTab).toHaveFocus()

    await user.keyboard('{ArrowRight}')
    expect(contextTab).toHaveAttribute('aria-selected', 'true')
    expect(contextTab).toHaveFocus()
    expect(changesTab).toHaveAttribute('tabindex', '-1')
  })

  it('selects changed files and navigates them with buttons and brackets', async () => {
    const user = userEvent.setup()
    renderApp()
    await userEvent.setup().click(
      await findReviewButton(),
    )

    await user.click(screen.getByRole('tab', { name: /^Changes/ }))
    expect(screen.getByText(/File 1 of 2/)).toHaveTextContent('src/client/screen/branch_sheet.rs')

    await user.click(screen.getByRole('button', { name: 'Next changed file' }))
    expect(screen.getByText(/File 2 of 2/)).toHaveTextContent('src/client/screen/frame_tests.rs')
    fireEvent.keyDown(window, { key: '[' })
    expect(screen.getByText(/File 1 of 2/)).toHaveTextContent('src/client/screen/branch_sheet.rs')

    await user.click(screen.getByRole('button', { name: 'Select changed file src/client/screen/frame_tests.rs' }))
    expect(screen.getByText(/File 2 of 2/)).toHaveTextContent('src/client/screen/frame_tests.rs')
  })

  it('does not navigate changed files while typing in a text field', async () => {
    const user = userEvent.setup()
    renderApp()
    await userEvent.setup().click(
      await findReviewButton(),
    )

    await user.click(screen.getByRole('tab', { name: /^Changes/ }))
    const composer = screen.getByRole('textbox', { name: 'Message Codex' })
    composer.focus()
    fireEvent.keyDown(composer, { key: ']' })

    expect(screen.getByText(/File 1 of 2/)).toBeInTheDocument()
  })

  it('restores a conversation draft after the workspace is reopened', async () => {
    const user = userEvent.setup()
    localStorage.clear()
    const app = renderApp()

    await user.click(screen.getByRole('button', { name: /^Review the daemon restore flow, Done, maitty$/i }))
    await user.type(screen.getByRole('textbox', { name: 'Message Claude' }), 'Keep this draft')
    app.unmount()
    renderApp()
    await user.click(screen.getByRole('button', { name: /^Review the daemon restore flow, Done, maitty$/i }))

    expect(screen.getByRole('textbox', { name: 'Message Claude' })).toHaveValue('Keep this draft')
  })

  it('restores the selected conversation after the workspace is reopened', async () => {
    const user = userEvent.setup()
    const app = renderApp()

    await user.click(screen.getByRole('button', { name: /^Review the daemon restore flow, Done, maitty$/i }))
    expect(screen.getByRole('heading', { name: 'Review the daemon restore flow' })).toBeInTheDocument()

    app.unmount()
    renderApp()

    expect(screen.getByRole('heading', { name: 'Review the daemon restore flow' })).toBeInTheDocument()
  })

  it('restores the selected theme after the workspace is reopened', async () => {
    const user = userEvent.setup()
    localStorage.clear()
    const app = renderApp()

    await user.click(screen.getByRole('button', { name: 'Open settings' }))
    await user.click(screen.getByRole('button', { name: /Paper/i }))
    expect(document.documentElement.dataset.theme).toBe('day')
    app.unmount()
    renderApp()

    expect(document.documentElement.dataset.theme).toBe('day')
  })

  it('opens Settings with the platform primary modifier and comma shortcut', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.keyboard('{Control>},{/Control}')

    expect(screen.getByRole('dialog')).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Settings' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Appearance' })).toBeInTheDocument()
  })

  it('requires a project folder instead of assuming a Windows path', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.keyboard('{Control>}n{/Control}')
    expect(screen.getByRole('textbox', { name: 'Project folder' })).toHaveValue('')
    await user.click(screen.getByRole('button', { name: /Start conversation/i }))

    expect(await screen.findByText('Choose a project folder first')).toBeInTheDocument()
  })

  it('opens a new conversation from the tab strip', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.click(screen.getByRole('button', { name: 'New tab' }))
    expect(screen.getByRole('dialog')).toHaveTextContent('New conversation')
  })

  it('pins, renames and closes sessions from their row menu', async () => {
    const user = userEvent.setup()
    localStorage.clear()
    const app = renderApp()

    await user.click(screen.getByRole('button', { name: /^Add a conversation reconnect test, Working, maitty$/i }))
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Add a conversation reconnect test,/ }))
    expect(screen.getByRole('menuitem', { name: 'Close session' })).toBeInTheDocument()
    await user.click(screen.getByRole('menuitem', { name: 'Pin conversation' }))
    expect(screen.getByRole('region', { name: 'maitty project' })).toContainElement(screen.getByRole('button', { name: /^Add a conversation reconnect test, Working, maitty$/i }))

    await user.click(screen.getByRole('button', { name: /^Review the daemon restore flow, Done, maitty$/i }))
    fireEvent.contextMenu(screen.getByRole('button', { name: /^Review the daemon restore flow,/ }))
    await user.click(screen.getByRole('menuitem', { name: 'Rename' }))
    const renameField = screen.getByRole('textbox', { name: 'Rename Review the daemon restore flow' })
    await user.clear(renameField)
    await user.type(renameField, 'Review daemon recovery')
    await user.keyboard('{Enter}')

    fireEvent.contextMenu(screen.getByRole('button', { name: /^Review daemon recovery,/ }))
    await user.click(screen.getByRole('menuitem', { name: 'Close session' }))
    await user.click(screen.getByRole('button', { name: 'End session' }))
    await waitFor(() => { expect(screen.queryByRole('button', { name: /^Review daemon recovery,/ })).not.toBeInTheDocument() })

    app.unmount()
    renderApp()
    expect(screen.getByRole('region', { name: 'maitty project' })).toContainElement(screen.getByRole('button', { name: /^Add a conversation reconnect test, Working, maitty$/i }))
    expect(screen.queryByRole('button', { name: /^Review daemon recovery,/ })).not.toBeInTheDocument()
  })

  describe('renaming a tab in place', () => {
    async function openTabRename(user: ReturnType<typeof userEvent.setup>) {
      localStorage.clear()
      renderApp()
      await user.click(screen.getByRole('button', { name: /^Review the daemon restore flow, Done, maitty$/i }))
      fireEvent.contextMenu(screen.getByRole('tab', { name: /Review the daemon restore flow/ }))
      await user.click(screen.getByRole('menuitem', { name: 'Rename' }))
      const field = await screen.findByRole('textbox', { name: 'Tab name' })
      await waitFor(() => { expect(field).toHaveFocus() })
      return field
    }

    it('saves a new name on Enter and shows it on the tab and the row', async () => {
      const user = userEvent.setup()
      const field = await openTabRename(user)
      expect(field).toHaveValue('Review the daemon restore flow')
      await user.clear(field)
      await user.type(field, 'Review daemon recovery{Enter}')

      expect(screen.queryByRole('textbox', { name: 'Tab name' })).not.toBeInTheDocument()
      expect(screen.getByRole('tab', { name: /Review daemon recovery/ })).toBeInTheDocument()
      expect(screen.getByRole('button', { name: /^Review daemon recovery,/ })).toBeInTheDocument()
    })

    it('keeps the old name on Escape', async () => {
      const user = userEvent.setup()
      const field = await openTabRename(user)
      await user.clear(field)
      await user.type(field, 'Never saved{Escape}')

      expect(screen.queryByRole('textbox', { name: 'Tab name' })).not.toBeInTheDocument()
      expect(screen.getByRole('tab', { name: /Review the daemon restore flow/ })).toBeInTheDocument()
      expect(screen.queryByText('Never saved')).not.toBeInTheDocument()
    })

    it('keeps the old name when the field is emptied', async () => {
      const user = userEvent.setup()
      const field = await openTabRename(user)
      await user.clear(field)
      await user.keyboard('{Enter}')

      expect(screen.queryByRole('textbox', { name: 'Tab name' })).not.toBeInTheDocument()
      expect(screen.getByRole('tab', { name: /Review the daemon restore flow/ })).toBeInTheDocument()
    })

    it('saves when focus leaves the field', async () => {
      const user = userEvent.setup()
      const field = await openTabRename(user)
      await user.clear(field)
      await user.type(field, 'Renamed on blur')
      await user.tab()

      expect(screen.getByRole('tab', { name: /Renamed on blur/ })).toBeInTheDocument()
    })
  })

  it('opens the next conversation that needs attention with Ctrl+Shift+A', async () => {
    const user = userEvent.setup()
    renderApp()
    await user.click(screen.getByRole('button', { name: /^Add a conversation reconnect test, Working, maitty$/i }))

    await user.keyboard('{Control>}{Shift>}a{/Shift}{/Control}')

    expect(screen.getByRole('button', { name: /tighten the branch view navigation, Needs you/i })).toHaveAttribute('aria-current', 'page')
  })

  it('moves through visible conversations with J/K and Alt+arrow keys', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.keyboard('j')
    expect(screen.getByRole('button', { name: /add a conversation reconnect test, Working/i })).toHaveAttribute('aria-current', 'page')

    await user.keyboard('{Alt>}{ArrowUp}{/Alt}')
    expect(screen.getByRole('button', { name: /tighten the branch view navigation, Needs you/i })).toHaveAttribute('aria-current', 'page')
  })

  it('leaves conversation shortcuts available for text entered in the search field', async () => {
    const user = userEvent.setup()
    renderApp()
    const search = screen.getByRole('textbox', { name: 'Search conversations' })
    await user.click(search)
    await user.type(search, 'j')

    expect(search).toHaveValue('j')
    expect(screen.getByRole('heading', { name: 'Tighten the branch view navigation' })).toBeInTheDocument()
  })

  it('answers the current approval with the Y shortcut', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.keyboard('y')

    expect(await screen.findByText('Approved once. The test command is running in this preview.')).toBeInTheDocument()
  })

  it('denies the pending request and sends the typed instructions instead', async () => {
    const user = userEvent.setup()
    renderApp()

    const box = screen.getByRole('textbox', { name: 'Message Codex' })
    expect(box).toBeEnabled()
    await user.type(box, 'Use the existing helper instead')
    await user.click(screen.getByRole('button', { name: 'Deny and send' }))

    expect(await screen.findByText('You declined the command. The agent did not run it.')).toBeInTheDocument()
    expect(await screen.findByText('Use the existing helper instead')).toBeInTheDocument()
  })

  it('starts a session in a typed folder path with Ctrl+O', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.keyboard('{Control>}o{/Control}')
    await user.type(screen.getByRole('textbox', { name: 'Project folder' }), '/home/casey/notes')

    expect(await screen.findByRole('textbox', { name: 'Project folder' })).toHaveValue('/home/casey/notes')
  })

  it('does not answer an approval when Y is typed into the search field', async () => {
    const user = userEvent.setup()
    renderApp()
    const search = screen.getByRole('textbox', { name: 'Search conversations' })
    await user.click(search)
    await user.type(search, 'y')

    expect(search).toHaveValue('y')
    expect(screen.getByRole('button', { name: 'Allow once' })).toBeInTheDocument()
  })

  it('shows an explicit approval decision in the conversation', async () => {
    const user = userEvent.setup()
    renderApp()

    await user.click(screen.getByRole('button', { name: 'Allow once' }))

    expect(screen.getByText('Request answered')).toBeInTheDocument()
    expect(screen.queryByText('Needs approval')).not.toBeInTheDocument()
  })

  it('shows the exact permissions and turn-scoped choices for a permission request', () => {
    const messages = previewMessages['codex-43']
    if (!messages) throw new Error('Preview conversation is missing')
    const message: Message = {
      id: 'test-permission-request',
      role: 'approval' as const,
      content: 'Codex is requesting additional permissions',
      time: 'Needs your decision',
      approvalKind: 'Permission' as const,
      approvalScope: 'C:/project',
      approvalPermissions: {
        network: { enabled: true },
        fileSystem: { write: ['C:/project/output'] },
      },
      approvalChoices: ['Decline', 'Accept'],
    }
    messages.push(message)
    try {
      renderApp()

      expect(screen.getByText('Allowing grants this exact permission profile for this turn only.')).toBeInTheDocument()
      expect(screen.getByText(/"write":/)).toBeInTheDocument()
      expect(screen.getByText('Requested from:')).toBeInTheDocument()
      expect(screen.getByRole('button', { name: 'Allow once' })).toBeInTheDocument()
      expect(screen.queryByRole('button', { name: 'Allow this session' })).not.toBeInTheDocument()
    } finally {
      messages.splice(messages.indexOf(message), 1)
    }
  })

  it('requires an answer to every Codex question before sending', async () => {
    const user = userEvent.setup()
    const messages = previewMessages['codex-43']
    if (!messages) throw new Error('Preview conversation is missing')
    const message = {
      id: 'test-user-input',
      role: 'approval' as const,
      content: 'Codex needs your input',
      time: 'Needs input',
      approvalKind: 'UserInput' as const,
      userQuestions: [
        {
          id: 'choice',
          header: 'Choice',
          question: 'Which option?',
          is_other: true,
          is_secret: false,
          options: [{ label: 'A', description: 'Choose A' }],
        },
        {
          id: 'secret',
          header: 'Secret',
          question: 'Enter private text',
          is_other: false,
          is_secret: true,
          options: [],
        },
      ],
    }
    messages.push(message)
    try {
      renderApp()

      const send = screen.getByRole('button', { name: 'Send answers' })
      expect(send).toBeDisabled()
      await user.click(screen.getByRole('checkbox', { name: /Choose A/ }))
      await user.type(screen.getByLabelText('Your answer for Secret'), 'private')
      expect(send).toBeEnabled()
      expect(screen.getByLabelText('Other answer for Choice')).toBeInTheDocument()
    } finally {
      messages.splice(messages.indexOf(message), 1)
    }
  })
})

async function findReviewButton(): Promise<HTMLElement> {
  // A session row can start with "Review" too, so match the header button by its class.
  return waitFor(() => {
    const button = document.querySelector<HTMLElement>('.review-button')
    expect(button).not.toBeNull()
    return button as HTMLElement
  })
}

function renderApp(client = new QueryClient({ defaultOptions: { queries: { retry: false } } })) {
  return render(<QueryClientProvider client={client}><App /></QueryClientProvider>)
}
