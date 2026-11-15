//! Teclado → `ShipInput`.

use raylib::prelude::{KeyboardKey, RaylibHandle};

use super::ShipInput;

/// `1` si solo está presionada `positive`, `-1` si solo `negative`.
fn axis(window: &RaylibHandle, positive: KeyboardKey, negative: KeyboardKey) -> f32 {
    let mut value = 0.0;
    if window.is_key_down(positive) {
        value += 1.0;
    }
    if window.is_key_down(negative) {
        value -= 1.0;
    }
    value
}

/// W/S: acelerar y frenar. Flechas: dirigir la punta. A/D: alabear. Shift: turbo.
pub fn read_input(window: &RaylibHandle) -> ShipInput {
    ShipInput {
        thrust: axis(window, KeyboardKey::KEY_W, KeyboardKey::KEY_S),
        pitch: axis(window, KeyboardKey::KEY_UP, KeyboardKey::KEY_DOWN),
        yaw: axis(window, KeyboardKey::KEY_RIGHT, KeyboardKey::KEY_LEFT),
        roll: axis(window, KeyboardKey::KEY_D, KeyboardKey::KEY_A),
        boost: window.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) || window.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT),
    }
}
