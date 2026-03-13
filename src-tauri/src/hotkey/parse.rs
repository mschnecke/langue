//! Hotkey string parsing (modifiers + key code)

use global_hotkey::hotkey::{Code, Modifiers};

use crate::error::AppError;

/// Parse modifier strings into Modifiers flags
pub fn parse_modifiers(mods: &[String]) -> Result<Modifiers, AppError> {
    let mut modifiers = Modifiers::empty();

    for m in mods {
        match m.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "meta" | "super" | "win" | "cmd" | "command" => modifiers |= Modifiers::META,
            "" => {}
            other => return Err(AppError::Hotkey(format!("Unknown modifier: {}", other))),
        }
    }

    Ok(modifiers)
}

/// Parse a key string into a Code
pub fn parse_code(key: &str) -> Result<Code, AppError> {
    let code = match key.to_uppercase().as_str() {
        // Letters
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,

        // Numbers
        "0" | "DIGIT0" => Code::Digit0,
        "1" | "DIGIT1" => Code::Digit1,
        "2" | "DIGIT2" => Code::Digit2,
        "3" | "DIGIT3" => Code::Digit3,
        "4" | "DIGIT4" => Code::Digit4,
        "5" | "DIGIT5" => Code::Digit5,
        "6" | "DIGIT6" => Code::Digit6,
        "7" | "DIGIT7" => Code::Digit7,
        "8" | "DIGIT8" => Code::Digit8,
        "9" | "DIGIT9" => Code::Digit9,

        // Function keys
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,

        // Special keys
        "SPACE" | " " => Code::Space,
        "ENTER" | "RETURN" => Code::Enter,
        "TAB" => Code::Tab,
        "ESCAPE" | "ESC" => Code::Escape,
        "BACKSPACE" => Code::Backspace,
        "DELETE" | "DEL" => Code::Delete,
        "INSERT" | "INS" => Code::Insert,
        "HOME" => Code::Home,
        "END" => Code::End,
        "PAGEUP" | "PGUP" => Code::PageUp,
        "PAGEDOWN" | "PGDN" => Code::PageDown,

        // Arrow keys
        "UP" | "ARROWUP" => Code::ArrowUp,
        "DOWN" | "ARROWDOWN" => Code::ArrowDown,
        "LEFT" | "ARROWLEFT" => Code::ArrowLeft,
        "RIGHT" | "ARROWRIGHT" => Code::ArrowRight,

        // Punctuation and symbols
        "MINUS" | "-" => Code::Minus,
        "EQUAL" | "=" => Code::Equal,
        "BRACKETLEFT" | "[" => Code::BracketLeft,
        "BRACKETRIGHT" | "]" => Code::BracketRight,
        "BACKSLASH" | "\\" => Code::Backslash,
        "SEMICOLON" | ";" => Code::Semicolon,
        "QUOTE" | "'" => Code::Quote,
        "BACKQUOTE" | "`" => Code::Backquote,
        "COMMA" | "," => Code::Comma,
        "PERIOD" | "." => Code::Period,
        "SLASH" | "/" => Code::Slash,

        // Numpad
        "NUMPAD0" => Code::Numpad0,
        "NUMPAD1" => Code::Numpad1,
        "NUMPAD2" => Code::Numpad2,
        "NUMPAD3" => Code::Numpad3,
        "NUMPAD4" => Code::Numpad4,
        "NUMPAD5" => Code::Numpad5,
        "NUMPAD6" => Code::Numpad6,
        "NUMPAD7" => Code::Numpad7,
        "NUMPAD8" => Code::Numpad8,
        "NUMPAD9" => Code::Numpad9,
        "NUMPADADD" | "NUMPAD+" => Code::NumpadAdd,
        "NUMPADSUBTRACT" | "NUMPAD-" => Code::NumpadSubtract,
        "NUMPADMULTIPLY" | "NUMPAD*" => Code::NumpadMultiply,
        "NUMPADDIVIDE" | "NUMPAD/" => Code::NumpadDivide,
        "NUMPADDECIMAL" | "NUMPAD." => Code::NumpadDecimal,
        "NUMPADENTER" => Code::NumpadEnter,

        other => return Err(AppError::Hotkey(format!("Unknown key: {}", other))),
    };

    Ok(code)
}
