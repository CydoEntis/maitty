# Tauri clipboard

> **Use for:** Reading a picture from the system clipboard in the Maitty desktop GUI.
> **Requires:** Tauri 2
> **Fills:** SECURITY § desktop capabilities

## Rules

- Read the clipboard only as the direct result of a paste the person made, never on a timer, on
  focus or in the background.
- Enable only the operations the GUI calls in its Tauri capability. Text is pasted by the web
  view's own paste event and needs no permission.
- What is read is treated as any pasted image is: checked by the host, kept in the private cache,
  never logged.

## Project-specific: clipboard

The GUI uses `@tauri-apps/plugin-clipboard-manager` and `tauri-plugin-clipboard-manager` 2.5.0.
The capability allows `clipboard-manager:allow-read-image` and nothing else.

It exists for one gap: WebKitGTK does not hand a pasted picture to the page (WebKit bug 218519),
so on Linux a paste of a picture arrives as an empty paste event. `api/clipboardImage.ts`
(`readClipboardImage`) then asks the host for the clipboard's picture, turns its pixels into a
PNG and hands it on as a pasted file; `pasteFromHost` in `features/terminal/clipboardImage.ts`
calls it from the chat box, the new-chat page and a pane. On Windows and macOS the web view's
paste event carries the picture and the host is never asked. On Wayland the plugin reads through
the compositor's data-control protocol.
