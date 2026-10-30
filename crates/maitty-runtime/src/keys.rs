pub use maitty_settings::keys::*;

/// Encodes mouse input for a pane only when its terminal has requested mouse reporting.
pub fn mouse_bytes(
    screen: &vt100::Screen,
    event: &crossterm::event::MouseEvent,
    col: u16,
    row: u16,
) -> Option<Vec<u8>> {
    use crossterm::event::{MouseButton as Button, MouseEventKind as Kind};
    use ratatui::crossterm::event::KeyModifiers;
    use vt100::{MouseProtocolEncoding as Encoding, MouseProtocolMode as Mode};

    let mode = screen.mouse_protocol_mode();
    if mode == Mode::None {
        return None;
    }
    let mut modifiers = 0u32;
    if event.modifiers.contains(KeyModifiers::SHIFT) {
        modifiers |= 4;
    }
    if event.modifiers.contains(KeyModifiers::ALT) {
        modifiers |= 8;
    }
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        modifiers |= 16;
    }
    let button_code = |button| match button {
        Button::Left => 0,
        Button::Middle => 1,
        Button::Right => 2,
    };
    let (code, release) = match event.kind {
        Kind::Down(button) => (button_code(button), false),
        Kind::Up(_) if mode == Mode::Press => return Some(Vec::new()),
        Kind::Up(button) => (button_code(button), true),
        Kind::Drag(button) if matches!(mode, Mode::ButtonMotion | Mode::AnyMotion) => {
            (32 + button_code(button), false)
        }
        Kind::Drag(_) => return Some(Vec::new()),
        Kind::Moved if mode == Mode::AnyMotion => (35, false),
        Kind::Moved => return None,
        Kind::ScrollUp => (64, false),
        Kind::ScrollDown => (65, false),
        Kind::ScrollLeft => (66, false),
        Kind::ScrollRight => (67, false),
    };
    let (x, y) = (col as u32 + 1, row as u32 + 1);
    Some(match screen.mouse_protocol_encoding() {
        Encoding::Sgr => format!(
            "\x1b[<{};{x};{y}{}",
            code + modifiers,
            if release { 'm' } else { 'M' }
        )
        .into_bytes(),
        encoding => {
            let code = if release {
                3 + modifiers
            } else {
                code + modifiers
            };
            let mut output = b"\x1b[M".to_vec();
            for value in [code, x, y] {
                let value = value + 32;
                if encoding == Encoding::Utf8 {
                    let mut buffer = [0; 4];
                    output.extend_from_slice(
                        char::from_u32(value)
                            .unwrap_or(' ')
                            .encode_utf8(&mut buffer)
                            .as_bytes(),
                    );
                } else {
                    output.push(value.min(255) as u8);
                }
            }
            output
        }
    })
}

#[cfg(test)]
mod tests {
    use super::mouse_bytes;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    fn event(kind: MouseEventKind) -> MouseEvent {
        MouseEvent {
            kind,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn forwards_mouse_events_only_for_the_terminal_modes_the_agent_requested() {
        let mut parser = vt100::Parser::new(10, 40, 0);
        assert_eq!(
            mouse_bytes(parser.screen(), &event(MouseEventKind::ScrollUp), 4, 2),
            None
        );
        parser.process(b"\x1b[?1002h\x1b[?1006h");
        assert_eq!(
            mouse_bytes(parser.screen(), &event(MouseEventKind::ScrollUp), 4, 2),
            Some(b"\x1b[<64;5;3M".to_vec())
        );
        assert_eq!(
            mouse_bytes(
                parser.screen(),
                &event(MouseEventKind::Down(MouseButton::Left)),
                0,
                0,
            ),
            Some(b"\x1b[<0;1;1M".to_vec())
        );
        assert_eq!(
            mouse_bytes(parser.screen(), &event(MouseEventKind::Moved), 0, 0),
            None
        );
        let mut x10 = vt100::Parser::new(10, 40, 0);
        x10.process(b"\x1b[?1000h");
        assert_eq!(
            mouse_bytes(x10.screen(), &event(MouseEventKind::ScrollDown), 0, 0),
            Some(vec![0x1b, b'[', b'M', 32 + 65, 33, 33])
        );
    }
}
