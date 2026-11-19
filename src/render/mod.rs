//! Renderer por software: de una lista de objetos a un framebuffer.
//!
//! Por cuadro: fondo → por cada objeto, vertex shader → recorte contra el
//! plano cercano → proyección → rasterización → prueba de profundidad →
//! fragment shader.

mod background;
mod camera;
mod clip;
mod context;
mod fragment;
mod framebuffer;
mod frustum;
mod pipeline;
mod raster;
mod sprites;
mod stats;

pub use background::Background;
pub use camera::{Camera, CameraRays, OrbitController};
pub use context::{FrameContext, ObjectContext};
pub use fragment::Fragment;
pub use framebuffer::Framebuffer;
pub use sprites::Sprite;
use frustum::Frustum;
pub use stats::RenderStats;

use crate::scene::SceneObject;

/// Dibuja el cuadro completo y devuelve cuánto trabajo hizo.
pub fn render_frame(
    framebuffer: &mut Framebuffer,
    background: &Background,
    objects: &[SceneObject],
    sprites: &[Sprite],
    frame: &FrameContext,
) -> RenderStats {
    let mut stats = RenderStats { objects_total: objects.len(), ..RenderStats::default() };

    background.paint(framebuffer, frame);
    framebuffer.clear_depth();

    let frustum = Frustum::from_view_projection(&frame.view_projection);
    for object in objects {
        // Si la esfera envolvente queda fuera de la vista, ningún triángulo del
        // objeto puede verse: se descarta sin transformar sus vértices.
        if frame.culling {
            let (center, radius) = pipeline::world_bounds(object);
            if !frustum.contains_sphere(center, radius) {
                stats.objects_culled += 1;
                continue;
            }
        }
        pipeline::draw_object(framebuffer, object, frame, &mut stats);
    }

    // Después de todo lo sólido, para que la profundidad ya esté completa.
    stats.sprites_drawn = sprites::draw_sprites(framebuffer, sprites, frame);

    stats
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use noise::OpenSimplex;
    use raylib::math::{Vector2, Vector3};

    use super::*;
    use crate::config::SPHERE_MODEL_PATH;
    use crate::math::Mat4;
    use crate::mesh::{Mesh, Normals, Obj, Vertex};
    use crate::shaders::Material;

    /// Suelo plano de 1000 × 1000 hecho con dos triángulos, mirando hacia `+y`.
    fn ground() -> SceneObject {
        let corner = |x: f32, z: f32| Vertex {
            position: Vector3::new(x, 0.0, z),
            normal: Vector3::new(0.0, 1.0, 0.0),
            uv: Vector2::zero(),
        };
        let (a, b, c, d) = (corner(-500.0, 500.0), corner(500.0, 500.0), corner(500.0, -500.0), corner(-500.0, -500.0));
        SceneObject {
            name: "suelo",
            mesh: Arc::new(Mesh::new(vec![a, b, c, a, c, d])),
            model: Mat4::IDENTITY,
            material: Material::Normals,
        }
    }

    /// Cámara a 0.15 sobre un suelo enorme, mirando 20° hacia abajo. Los dos
    /// triángulos tienen vértices detrás de la cámara: sin recorte se
    /// descartaban completos y la mitad inferior de la pantalla quedaba vacía.
    /// El horizonte está por encima del centro, así que la mitad inferior debe
    /// quedar cubierta por completo.
    #[test]
    fn ground_has_no_holes_with_camera_just_above_it() {
        let (width, height) = (320, 180);
        let noise = OpenSimplex::new(1);
        let background = Background::new(&noise);
        let mut framebuffer = Framebuffer::new(width, height);
        let ground = ground();
        let looking_down = 20f32.to_radians();

        for step in 0..24 {
            let yaw = step as f32 * std::f32::consts::TAU / 24.0;
            let forward = Vector3::new(yaw.sin() * looking_down.cos(), -looking_down.sin(), yaw.cos() * looking_down.cos());
            let position = Vector3::new(0.0, 0.15, 0.0);
            let camera = Camera {
                position,
                target: position + forward,
                up: Vector3::new(0.0, 1.0, 0.0),
                field_of_view: 60f32.to_radians(),
                near: 0.1,
                far: 100.0,
            };

            let frame = FrameContext::new(&camera, width, height, 0.0, Vector3::new(0.0, 50.0, 0.0), &noise);
            let stats = render_frame(&mut framebuffer, &background, std::slice::from_ref(&ground), &[], &frame);
            assert!(stats.triangles_clipped > 0, "el paso {step} no ejercita el recorte");

            let lower_half = &framebuffer.depths[width * height / 2..];
            let holes = lower_half.iter().filter(|depth| **depth <= 0.0).count();
            assert_eq!(holes, 0, "{holes} píxeles sin suelo en el paso {step}");
        }
    }

    /// Un objeto completamente detrás de la cámara no produce fragmentos.
    #[test]
    fn objects_behind_the_camera_are_discarded() {
        let noise = OpenSimplex::new(1);
        let mut framebuffer = Framebuffer::new(64, 64);
        let camera = Camera {
            position: Vector3::new(0.0, 5.0, 0.0),
            target: Vector3::new(0.0, 10.0, 0.0),
            up: Vector3::new(0.0, 0.0, 1.0),
            field_of_view: 1.0,
            near: 0.1,
            far: 100.0,
        };
        let frame = FrameContext::new(&camera, 64, 64, 0.0, Vector3::zero(), &noise);
        let stats = render_frame(&mut framebuffer, &Background::new(&noise), std::slice::from_ref(&ground()), &[], &frame);

        assert_eq!(stats.triangles_behind_camera, 2);
        assert_eq!(stats.fragments_rasterized, 0);
    }

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
            let stats = render_frame(&mut framebuffer, &background, std::slice::from_ref(&sphere), &[], &frame);
            assert!(stats.fragments_shaded > 0, "la esfera no se dibujó en el paso {step}");

            for row in framebuffer.depths.chunks_exact(width) {
                let covered = |depth: &f32| *depth > 0.0;
                if let (Some(first), Some(last)) = (row.iter().position(covered), row.iter().rposition(covered)) {
                    assert!(row[first..=last].iter().all(covered), "hueco en el paso {step}");
                }
            }
        }
    }
}
