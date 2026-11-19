use noise::OpenSimplex;
use raylib::math::Vector3;

use super::{Camera, CameraRays};
use crate::math::Mat4;

/// Lo que es igual para todos los fragmentos de un objeto en un cuadro.
#[derive(Clone, Copy, Debug)]
pub struct ObjectContext {
    /// Dirección del origen del objeto hacia el sol, en el espacio del objeto.
    /// Para un planeta el sol está tan lejos que sirve para toda su superficie,
    /// y no cambia aunque el planeta gire: es la que necesitan los patrones que
    /// se calculan con `fragment.object_position`.
    pub sun_direction: Vector3,
    /// Qué tan cerca está la nave: 0 lejos, 1 pegada a la superficie. Ver
    /// `scene::proximity`.
    pub proximity: f32,
    /// Dirección del centro del objeto hacia la nave, en el espacio del objeto
    /// y con largo 1. En una esfera es el punto de la superficie que queda
    /// justo debajo de la nave: de ahí salen los efectos que reaccionan a ella.
    pub ship_direction: Vector3,
}

/// Lo que es igual para todos los objetos de un cuadro. Lo que cambia por
/// objeto (matriz de modelo, material) viaja en `SceneObject`.
#[allow(dead_code, reason = "API pública que todavía no usa la escena de prueba")]
pub struct FrameContext {
    pub view: Mat4,
    pub projection: Mat4,
    /// `projection * view`, calculada una vez por cuadro.
    pub view_projection: Mat4,
    pub camera_position: Vector3,
    pub rays: CameraRays,
    /// Distancia al plano cercano: ahí se recortan los triángulos.
    pub near: f32,
    pub width: usize,
    pub height: usize,
    /// Segundos de simulación. Todo lo animado depende de esto, no del número
    /// de cuadro.
    pub time: f32,
    /// El sol es una luz puntual: cada fragmento calcula su propia dirección.
    pub light_position: Vector3,
    /// Dónde está la nave. Los planetas reaccionan cuando se acerca. Mientras
    /// no se indique con `with_ship`, es la posición de la cámara.
    pub ship_position: Vector3,
    /// Si se descartan los objetos que quedan fuera de la vista. Se apaga para
    /// comparar cuánto trabajo ahorra.
    pub culling: bool,
    pub noise: OpenSimplex,
}

impl FrameContext {
    pub fn new(
        camera: &Camera,
        width: usize,
        height: usize,
        time: f32,
        light_position: Vector3,
        noise: &OpenSimplex,
    ) -> Self {
        let view = camera.view();
        let projection = camera.projection(width as f32 / height as f32);

        FrameContext {
            view,
            projection,
            view_projection: projection * view,
            camera_position: camera.position,
            rays: camera.rays(width, height),
            near: camera.near,
            width,
            height,
            time,
            light_position,
            ship_position: camera.position,
            culling: true,
            noise: *noise,
        }
    }

    pub fn with_culling(mut self, culling: bool) -> Self {
        self.culling = culling;
        self
    }

    pub fn with_ship(mut self, ship_position: Vector3) -> Self {
        self.ship_position = ship_position;
        self
    }
}
