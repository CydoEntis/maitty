# Tauri dialog

> **Use for:** Native folder selection in the Maitty desktop GUI.
> **Requires:** Tauri 2
> **Fills:** SECURITY § desktop capabilities

## Rules

- Use the operating system's native dialog for user-selected folders.
- Enable only the dialog operation the GUI calls in its Tauri capability.
- Treat a canceled dialog as no selection. Do not store or log paths selected by
  the user beyond the feature that needs them.
- Do not add filesystem plugin permissions to support a native picker.

## Project-specific: dialog

The GUI uses `@tauri-apps/plugin-dialog` and `tauri-plugin-dialog` 2.8.4 to
select one project directory. The capability allows only `dialog:allow-open`.
