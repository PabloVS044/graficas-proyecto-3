use noise::OpenSimplex;
use raylib::math::Vector3;

use super::{Camera, CameraRays};
use crate::math::Mat4;

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
    pub width: usize,
    pub height: usize,
    /// Segundos de simulación. Todo lo animado depende de esto, no del número
    /// de cuadro.
    pub time: f32,
    /// El sol es una luz puntual: cada fragmento calcula su propia dirección.
    pub light_position: Vector3,
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
            width,
            height,
            time,
            light_position,
            noise: *noise,
        }
    }
}
