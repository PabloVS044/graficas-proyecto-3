//! Apariencia provisional de un planeta: un tablero de dos colores sobre la
//! esfera. Sirve para ver que orbita y que gira sobre su eje mientras se le
//! hace su shader propio.

use raylib::math::Vector3;

use super::{light_direction, mix};
use crate::render::{Fragment, FrameContext};

/// Gajos a lo largo del ecuador y franjas de polo a polo.
const SEGMENTS: f32 = 12.0;
const BANDS: f32 = 6.0;
const AMBIENT: f32 = 0.06;

pub fn shade(fragment: &Fragment, frame: &FrameContext, base: Vector3, accent: Vector3) -> Vector3 {
    let p = fragment.object_position.normalize();
    let longitude = p.z.atan2(p.x) / std::f32::consts::TAU + 0.5;
    let latitude = p.y.clamp(-1.0, 1.0).asin() / std::f32::consts::PI + 0.5;

    let cell = (longitude * SEGMENTS).floor() as i32 + (latitude * BANDS).floor() as i32;
    let albedo = mix(base, accent, (cell & 1) as f32);

    let diffuse = fragment.normal.dot(light_direction(fragment, frame)).max(0.0);
    albedo * (AMBIENT + diffuse * (1.0 - AMBIENT))
}
