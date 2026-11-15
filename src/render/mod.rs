//! Renderer por software: de una lista de objetos a un framebuffer.
//!
//! Por cuadro: fondo → por cada objeto, vertex shader → rasterización →
//! prueba de profundidad → fragment shader.

mod background;
mod camera;
mod context;
mod fragment;
mod framebuffer;
mod pipeline;
mod raster;
mod stats;

pub use background::Background;
pub use camera::{Camera, CameraRays, OrbitController};
pub use context::FrameContext;
pub use fragment::Fragment;
pub use framebuffer::Framebuffer;
pub use stats::RenderStats;

use crate::scene::SceneObject;

/// Dibuja el cuadro completo y devuelve cuánto trabajo hizo.
pub fn render_frame(
    framebuffer: &mut Framebuffer,
    background: &Background,
    objects: &[SceneObject],
    frame: &FrameContext,
) -> RenderStats {
    let mut stats = RenderStats { objects_total: objects.len(), ..RenderStats::default() };

    background.paint(framebuffer, frame);
    framebuffer.clear_depth();

    for object in objects {
        pipeline::draw_object(framebuffer, object, frame, &mut stats);
    }

    stats
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use noise::OpenSimplex;
    use raylib::math::Vector3;

    use super::*;
    use crate::config::SPHERE_MODEL_PATH;
    use crate::math::Mat4;
    use crate::mesh::{Normals, Obj};
    use crate::shaders::Material;

    /// La esfera es convexa: en cada fila, los píxeles entre el primero y el
    /// último que la cubren también deben estar cubiertos, desde cualquier ángulo.
    #[test]
    fn sphere_has_no_holes_from_any_angle() {
        let (width, height) = (320, 180);
        let noise = OpenSimplex::new(1);
        let background = Background::new(&noise);
        let mut framebuffer = Framebuffer::new(width, height);
        let sphere = SceneObject {
            name: "esfera",
            mesh: Arc::new(Obj::load(SPHERE_MODEL_PATH).unwrap().to_mesh(Normals::Smooth)),
            model: Mat4::IDENTITY,
            material: Material::Normals,
        };

        for step in 0..100 {
            let orbit = OrbitController {
                target: Vector3::zero(),
                yaw: step as f32 * 0.137,
                pitch: (step as f32 * 0.31).sin() * 1.4,
                distance: 2.0 + (step % 5) as f32 * 0.5,
            };
            let camera = orbit.camera(1.0, 0.1, 100.0);
            let frame = FrameContext::new(&camera, width, height, 0.0, Vector3::new(5.0, 5.0, 5.0), &noise);
            let stats = render_frame(&mut framebuffer, &background, std::slice::from_ref(&sphere), &frame);
            assert!(stats.fragments_shaded > 0, "la esfera no se dibujó en el paso {step}");

            for row in framebuffer.depths.chunks_exact(width) {
                let covered = |depth: &f32| depth.is_finite();
                if let (Some(first), Some(last)) = (row.iter().position(covered), row.iter().rposition(covered)) {
                    assert!(row[first..=last].iter().all(covered), "hueco en el paso {step}");
                }
            }
        }
    }
}
