import eslint from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'

// Hard-coded colours (hex, rgb(), hsl()) bypass the theme tokens (docs/UI.md, "Design system rules").
const COLOR_LITERAL = String.raw`/(#[0-9a-f]{3,8}\b|\b(rgb|hsl)a?\()/i`
const HARD_CODED_COLOR = [
  { selector: `Literal[value=${COLOR_LITERAL}]`, message: 'Use a theme token (CSS variable), not a hard-coded colour.' },
  { selector: `TemplateElement[value.raw=${COLOR_LITERAL}]`, message: 'Use a theme token (CSS variable), not a hard-coded colour.' },
]

// A control that Radix already builds (docs/stack/shadcn-tailwind.md) is not built again by hand: it comes from
// `src/components/ui/`, where it is written once and reads the theme tokens, so every window style and theme
// reaches every instance of it. These are the roles such a control carries.
const HAND_BUILT_ROLES = String.raw`/^(radiogroup|radio|switch|dialog|alertdialog|menu|menuitem|menuitemradio|listbox|option)$/`
const HAND_BUILT_CONTROL_MESSAGE =
  'Use the shared component from src/components/ui (Dialog, AlertDialog, DropdownMenu, ContextMenu, Select, Switch, ToggleGroup, Tooltip), not a hand-built one: docs/stack/shadcn-tailwind.md.'
const HAND_BUILT_CONTROL = [
  { selector: `JSXAttribute[name.name='role'][value.value=${HAND_BUILT_ROLES}]`, message: HAND_BUILT_CONTROL_MESSAGE },
  { selector: "ImportDeclaration[source.value='react-dom'] ImportSpecifier[imported.name='createPortal']", message: 'A portal belongs to the Radix primitive in src/components/ui; do not open one by hand.' },
]

export default tseslint.config(
  { ignores: ['dist/**', 'node_modules/**', 'target/**', 'src-tauri/target/**', 'gui/**', 'eslint.config.ts', 'scripts/perf/**'] },
  eslint.configs.recommended,
  ...tseslint.configs.strictTypeChecked.map((config) => ({
    ...config,
    files: ['**/*.{ts,tsx}'],
  })),
  {
    files: ['vite.config.ts', 'scripts/**/*.ts'],
    languageOptions: {
      globals: globals.node,
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname },
    },
  },
  {
    files: ['src/**/*.{ts,tsx}'],
    languageOptions: {
      globals: { ...globals.browser, ...globals.es2022 },
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname },
    },
    plugins: {
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
    },
    rules: {
      ...reactHooks.configs.flat.recommended.rules,
      'react-refresh/only-export-components': ['error', { allowConstantExport: true }],
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-floating-promises': 'error',
      '@typescript-eslint/no-misused-promises': 'error',
      '@typescript-eslint/switch-exhaustiveness-check': 'error',
      '@typescript-eslint/no-non-null-assertion': 'error',
      '@typescript-eslint/no-magic-numbers': ['error', {
        ignore: [-1, 0, 1],
        ignoreArrayIndexes: true,
        ignoreDefaultValues: true,
        ignoreEnums: true,
        ignoreNumericLiteralTypes: true,
        ignoreReadonlyClassProperties: true,
        ignoreTypeIndexes: true,
      }],
    },
  },
  {
    files: ['src/**/*.{ts,tsx}'],
    ignores: [
      'src/**/*.{test,spec}.{ts,tsx}',
      // The theme files own the palettes and colour maths.
      'src/features/workspace/theme/**',
      // The terminal palette is not a theme token: xterm.js takes literal colours.
      'src/TerminalView.tsx',
    ],
    rules: { 'no-restricted-syntax': ['error', ...HARD_CODED_COLOR, ...HAND_BUILT_CONTROL] },
  },
  {
    // The ratchet: these files still hold a hand-built control and are migrated to src/components/ui one at a
    // time. Nothing is added to this list; a file leaves it when its last hand-built control is replaced.
    files: [
      'src/features/chat/components/AgentPicker.tsx',
      'src/features/crew/components/CrewChat.tsx',
      'src/features/inspect/HistoryTab.tsx',
      'src/features/settings/components/DesktopSettings.tsx',
      'src/features/settings/components/ThemePicker.tsx',
      'src/features/updates/UpdateSlot.tsx',
      'src/features/workspace/DesktopApp.tsx',
      'src/features/workspace/components/CommandPalette.tsx',
      'src/features/workspace/components/ConfirmCloseDialog.tsx',
      'src/features/workspace/components/GetFileDialog.tsx',
      'src/features/workspace/components/HoverTips.tsx',
      'src/features/workspace/components/InboxPanel.tsx',
      'src/features/workspace/components/MachineUpdateDialog.tsx',
      'src/features/workspace/components/MenuPopup.tsx',
      'src/features/workspace/components/NewChatMenus.tsx',
      'src/features/workspace/components/NewSessionModal.tsx',
      'src/features/workspace/components/RenameDialog.tsx',
      'src/features/workspace/components/ServiceStatus.tsx',
      'src/features/worktrees/NewWorktreeDialog.tsx',
    ],
    rules: { 'no-restricted-syntax': ['error', ...HARD_CODED_COLOR] },
  },
  {
    // The shared components are where the controls are built, once.
    files: ['src/components/ui/**/*.{ts,tsx}'],
    rules: { 'no-restricted-syntax': ['error', ...HARD_CODED_COLOR] },
  },
  {
    files: ['src/**/*.{test,spec}.{ts,tsx}'],
    rules: { '@typescript-eslint/no-magic-numbers': 'off' },
  },
  {
    files: ['src/components/ui/**/*.{ts,tsx}'],
    rules: { 'react-refresh/only-export-components': 'off' },
  },
)
