import type { SlashCommand } from './slashCommands'

/** A command whose result the conversation shows: it is sent from the chat like a message. */
const said = (name: string, description: string, hint = ''): SlashCommand => ({ name, description, hint, aliases: [] })
/** A command that opens a picker, a page or a notice only Codex's terminal draws: it is run there, with the terminal shown. */
const shown = (name: string, description: string, hint = '', aliases: readonly string[] = []): SlashCommand => ({ name, description, hint, aliases, terminal: true })

/**
 * Codex's own commands. Codex has no way to be asked what it takes (Claude Code has: `agent_commands`), so they are
 * written here by hand: the names from OpenAI's command reference and the descriptions from the Codex CLI 0.167.0
 * installed where this was written. A command Codex adds later is missing until this list is brought up to date,
 * and one it drops is answered by Codex itself. Windows-only and debugging commands are left out.
 */
export const CODEX_COMMANDS: readonly SlashCommand[] = [
  said('clear', 'Clear the terminal and start a new chat'),
  said('compact', 'Summarize the conversation to prevent hitting the context limit'),
  said('init', 'Create an AGENTS.md file with instructions for Codex'),
  said('new', 'Start a new chat during a conversation'),
  said('plan', 'Switch to Plan mode', '[prompt]'),
  said('review', 'Review my current changes and find issues'),
  shown('agents', 'Open the agent command center'),
  shown('app', 'Continue this session in the Desktop app'),
  shown('approve', 'Approve one retry of a recent auto-review denial'),
  shown('apps', 'Manage apps'),
  shown('archive', 'Archive this session'),
  shown('cd', 'Change the current working directory', '<path>'),
  shown('config', 'Configure Codex preferences'),
  shown('copy', 'Copy the last response or part of it'),
  shown('daemon', 'Manage the local background server'),
  shown('daybreak', 'Turn Daybreak on or off'),
  shown('debug-config', 'Show config layers and requirement sources for debugging'),
  shown('delete', 'Permanently delete this session'),
  shown('diff', 'Show git diff (including untracked files)'),
  shown('experimental', 'Toggle experimental features'),
  shown('export', 'Export the conversation as markdown'),
  shown('fast', 'Toggle the Fast service tier, when the model has one'),
  shown('feedback', 'Send logs to maintainers'),
  shown('fork', 'Fork the current chat'),
  shown('goal', 'Set or view the goal for a long-running task'),
  shown('hooks', 'View and manage lifecycle hooks'),
  shown('ide', 'Include current selection, open files, and other context from your IDE'),
  shown('import', 'Import setup, this project, and recent chats from Claude Code'),
  shown('keymap', 'Remap TUI shortcuts'),
  shown('logout', 'Sign out of Codex'),
  shown('mcp', 'List MCP tools; use /mcp verbose or /mcp login <name>'),
  shown('memories', 'Configure memory use and generation'),
  shown('mention', 'Mention a file'),
  shown('model', 'Choose what model and reasoning effort to use'),
  shown('permissions', 'Choose what Codex is allowed to do'),
  shown('pets', 'Choose or hide the terminal pet', '', ['pet']),
  shown('plugins', 'Browse plugins'),
  shown('presets', 'Run or save reusable prompts and slash commands'),
  shown('ps', 'List background terminals'),
  shown('pwd', 'Show the current working directory', '', ['cwd']),
  shown('quit', 'Exit Codex', '', ['exit']),
  shown('raw', 'Toggle raw scrollback mode for copy-friendly terminal selection'),
  shown('recap', 'Summarize the current conversation now'),
  shown('rename', 'Rename the current thread'),
  shown('resume', 'Resume a saved chat'),
  shown('side', 'Start a side conversation in an ephemeral fork', '', ['btw']),
  shown('skills', 'Use skills to improve how Codex performs specific tasks'),
  shown('status', 'Show current session configuration and token usage'),
  shown('statusline', 'Configure which items appear in the status line'),
  shown('stop', 'Stop all background terminals', '', ['clean']),
  shown('subagents', "Switch between this session's subagents"),
  shown('theme', 'Choose a syntax highlighting theme'),
  shown('title', 'Configure which items appear in the terminal title'),
  shown('tui', 'Choose the TUI mode for the next launch'),
  shown('usage', 'View account usage or use a usage limit reset'),
  shown('vim', 'Toggle Vim mode for the composer'),
  shown('voice', 'Start or stop voice; use /voice settings to choose a voice'),
  shown('warnings', 'View retained warnings and diagnostic details'),
  shown('worktree', 'Start or continue a conversation in a new worktree'),
]
