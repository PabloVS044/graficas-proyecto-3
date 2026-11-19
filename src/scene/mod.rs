//! Escena: los objetos que se dibujan en un cuadro.

pub mod planets;
mod solar_system;
mod surface;

pub use solar_system::{Detail, SolarSystem};

use std::sync::Arc;

use raylib::math::Vector3;

use crate::math::Mat4;
use crate::mesh::Mesh;
use crate::shaders::{Material, smoothstep};

/// Desde qué altura, medida en radios del cuerpo, empieza a notarse la nave.
pub const PROXIMITY_RANGE: f32 = 4.0;

/// Qué tan cerca está la nave de un cuerpo esférico: 0 a más de
/// `PROXIMITY_RANGE` radios de altura, 1 pegada a la superficie. La altura se
/// mide en radios para que un planeta grande se note desde más lejos.
pub fn proximity(distance_to_center: f32, radius: f32) -> f32 {
    if radius <= 0.0 {
        return 0.0;
    }
    let altitude = (distance_to_center - radius).max(0.0) / radius;
    1.0 - smoothstep(0.15, PROXIMITY_RANGE, altitude)
}

/// Un objeto listo para dibujar. La malla se comparte entre objetos (muchos
/// árboles, una sola malla de árbol), por eso va en `Arc`.
#[allow(dead_code, reason = "el nombre sirve para depurar")]
pub struct SceneObject {
    pub name: &'static str,
    pub mesh: Arc<Mesh>,
    /// Objeto → mundo. Usar escala uniforme: las normales se transforman con
    /// esta misma matriz.
    pub model: Mat4,
    pub material: Material,
}

/// Esfera sólida que la nave no puede atravesar.
#[derive(Clone, Copy, Debug)]
pub struct Collider {
    pub name: &'static str,
    pub center: Vector3,
    pub radius: f32,
    /// Velocidad del cuerpo: un planeta se mueve en su órbita, y la colisión se
    /// resuelve con la velocidad relativa.
    pub velocity: Vector3,
    /// Aceleración de la gravedad en la superficie. Más lejos cae con el
    /// cuadrado de la distancia al centro.
    pub surface_gravity: f32,
}

/// Gravedad en la superficie por cada unidad de radio. Con la misma densidad,
/// la gravedad en la superficie crece con el radio: un planeta grande jala más.
pub const GRAVITY_PER_RADIUS: f32 = 0.65;

/// Escala que deja a la malla con el radio pedido, centrada en el origen del objeto.
pub fn fit_to_radius(mesh: &Mesh, radius: f32) -> Mat4 {
    Mat4::scale(radius / mesh.bounds.radius) * Mat4::translation(-mesh.bounds.center)
}
