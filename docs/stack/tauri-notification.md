# Tauri notification

> **Use for:** Native system notifications from the Maitty desktop GUI.
> **Requires:** Tauri 2
> **Fills:** SECURITY § desktop capabilities

## Rules

- Send a notification only for a state change the user asked to hear about (Settings ›
  Notifications has one switch each), and only for a session they are not looking at.
- A notification names the session and where it is. It never carries what an agent was asked or
  is asking, a command, a path of a file, or any secret: it can show on a locked screen.
- Enable only the operations the GUI calls in its Tauri capability.
- Ask the system for permission once per run and remember a refusal; a denied app must not nag.
- Do not use notifications as the only way to learn a state. The window's own badges and the
  sidebar say the same thing.

## Project-specific: notification

The GUI uses `@tauri-apps/plugin-notification` and `tauri-plugin-notification` 2.8.0. The
capability allows `notification:allow-is-permission-granted`, `notification:allow-request-permission`
and `notification:allow-notify`. The desktop plugin has no action buttons, so a click on a
notification only brings the window forward; the window then shows the session it was about if
that was within a minute. Answering an approval stays in the session's chat.
