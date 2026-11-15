//! Shaders sencillos para revisar el pipeline. Los de los planetas van en
//! archivos propios.

use raylib::math::Vector3;

use super::light_direction;
use crate::render::{Fragment, FrameContext};

/// Luz mínima del lado oscuro, para que no quede negro puro.
const AMBIENT: f32 = 0.06;

pub fn lambert(albedo: Vector3, fragment: &Fragment, frame: &FrameContext) -> Vector3 {
    let diffuse = fragment.normal.dot(light_direction(fragment, frame)).max(0.0);
    albedo * (AMBIENT + diffuse * (1.0 - AMBIENT))
}

/// Normal de `[-1, 1]` a color de `[0, 1]`.
pub fn normals(fragment: &Fragment) -> Vector3 {
    (fragment.normal + Vector3::one()) * 0.5
}
