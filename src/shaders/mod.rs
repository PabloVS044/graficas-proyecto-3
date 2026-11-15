//! Fragment shaders. Cada objeto de la escena lleva un `Material`, y `shade`
//! elige el shader según la variante.
//!
//! Para agregar un shader: una variante nueva en `Material` (con sus
//! parámetros), un archivo en esta carpeta con su función `shade`, y una rama
//! en el `match` de abajo.

mod basic;
mod placeholder;
mod ship;
pub mod noise_helpers;

use raylib::math::Vector3;

use crate::render::{Fragment, FrameContext};

/// Cómo se ve un objeto.
#[derive(Clone, Copy, Debug)]
pub enum Material {
    /// Difuso (Lambert) iluminado por el sol. Útil para objetos sencillos.
    #[allow(dead_code, reason = "API pública que todavía no usa la escena")]
    Lambert { albedo: Vector3 },
    /// Emite su propio color, no recibe luz. Provisional para el sol.
    Emissive { color: Vector3 },
    /// Provisional para planetas: tablero de dos colores, iluminado por el sol.
    Placeholder { base: Vector3, accent: Vector3 },
    /// Casco desgastado con franjas rojas y motores encendidos.
    /// `throttle` va de 0 (reposo) a 1 (turbo).
    Ship { throttle: f32 },
    /// Pinta la normal como color. Sirve para revisar geometría.
    #[allow(dead_code, reason = "API pública que todavía no usa la escena de prueba")]
    Normals,
}

pub fn shade(material: &Material, fragment: &Fragment, frame: &FrameContext) -> Vector3 {
    match *material {
        Material::Lambert { albedo } => basic::lambert(albedo, fragment, frame),
        Material::Emissive { color } => color,
        Material::Placeholder { base, accent } => placeholder::shade(fragment, frame, base, accent),
        Material::Ship { throttle } => ship::shade(fragment, frame, throttle),
        Material::Normals => basic::normals(fragment),
    }
}

/// Atajo para escribir un color con tres componentes.
pub const fn color(r: f32, g: f32, b: f32) -> Vector3 {
    Vector3 { x: r, y: g, z: b }
}

/// Interpolación lineal entre dos colores.
pub fn mix(a: Vector3, b: Vector3, amount: f32) -> Vector3 {
    a * (1.0 - amount) + b * amount
}

/// 0 antes de `start`, 1 después de `end` y una transición suave entre ambos.
pub fn smoothstep(start: f32, end: f32, value: f32) -> f32 {
    let t = ((value - start) / (end - start)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Dirección normalizada del fragmento hacia el sol.
pub fn light_direction(fragment: &Fragment, frame: &FrameContext) -> Vector3 {
    (frame.light_position - fragment.world_position).normalize()
}
