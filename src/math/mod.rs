//! Matemática propia del renderer.
//!
//! De raylib solo se usan `Vector2`, `Vector3` y `Vector4` como contenedores
//! (suma, producto punto, producto cruz, normalizar). Las matrices de modelo,
//! vista y proyección y las rotaciones con cuaterniones están escritas aquí,
//! para no depender de la cámara ni de las transformaciones de raylib.

mod mat4;
mod quat;

pub use mat4::Mat4;
pub use quat::Quat;
