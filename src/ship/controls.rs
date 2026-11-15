//! Teclado → `ShipInput`.

use raylib::prelude::{KeyboardKey, RaylibHandle};

use super::ShipInput;

/// Cómo responden las flechas arriba y abajo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PitchControl {
    /// Como la palanca de un avión: abajo (halar) sube la punta, arriba
    /// (empujar) la baja.
    Aircraft,
    /// Arriba sube la punta, abajo la baja.
    Direct,
}

impl PitchControl {
    pub fn toggled(self) -> PitchControl {
        match self {
            PitchControl::Aircraft => PitchControl::Direct,
            PitchControl::Direct => PitchControl::Aircraft,
        }
    }

    /// Nombre para el HUD.
    pub fn label(self) -> &'static str {
        match self {
            PitchControl::Aircraft => "avion (abajo sube)",
            PitchControl::Direct => "directo (arriba sube)",
        }
    }
}

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
pub fn read_input(window: &RaylibHandle, pitch_control: PitchControl) -> ShipInput {
    let (nose_up, nose_down) = match pitch_control {
        PitchControl::Aircraft => (KeyboardKey::KEY_DOWN, KeyboardKey::KEY_UP),
        PitchControl::Direct => (KeyboardKey::KEY_UP, KeyboardKey::KEY_DOWN),
    };

    ShipInput {
        thrust: axis(window, KeyboardKey::KEY_W, KeyboardKey::KEY_S),
        pitch: axis(window, nose_up, nose_down),
        yaw: axis(window, KeyboardKey::KEY_RIGHT, KeyboardKey::KEY_LEFT),
        roll: axis(window, KeyboardKey::KEY_D, KeyboardKey::KEY_A),
        boost: window.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) || window.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT),
    }
}
