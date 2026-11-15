//! Escena: los objetos que se dibujan en un cuadro.

mod demo;

pub use demo::DemoScene;

use std::sync::Arc;

use crate::math::Mat4;
use crate::mesh::Mesh;
use crate::shaders::Material;

/// Un objeto listo para dibujar. La malla se comparte entre objetos (muchos
/// árboles, una sola malla de árbol), por eso va en `Arc`.
#[allow(dead_code, reason = "API pública que todavía no usa la escena de prueba")]
pub struct SceneObject {
    pub name: &'static str,
    pub mesh: Arc<Mesh>,
    /// Objeto → mundo. Usar escala uniforme: las normales se transforman con
    /// esta misma matriz.
    pub model: Mat4,
    pub material: Material,
}

/// Escala que deja a la malla con el radio pedido, centrada en el origen del objeto.
pub fn fit_to_radius(mesh: &Mesh, radius: f32) -> Mat4 {
    Mat4::scale(radius / mesh.bounds.radius) * Mat4::translation(-mesh.bounds.center)
}
