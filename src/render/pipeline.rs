//! Las etapas que recorre cada objeto: vertex shader, recorte contra el plano
//! cercano, proyección a pantalla, rasterización, prueba de profundidad y
//! fragment shader.

use rayon::prelude::*;
use raylib::math::{Vector2, Vector3, Vector4};

use super::clip::{self, ClipVertex, Clipped};
use super::fragment::Fragment;
use super::raster::{self, Outcome};
use super::{FrameContext, Framebuffer, ObjectContext, RenderStats};
use crate::math::Mat4;
use crate::mesh::Vertex;
use crate::scene::{self, SceneObject};
use crate::shaders;

/// Vértice ya proyectado: lo que el rasterizador necesita.
#[derive(Clone, Copy, Debug)]
pub struct ScreenVertex {
    /// Posición en píxeles.
    pub x: f32,
    pub y: f32,
    /// `1 / w`. Es la profundidad (más grande = más cerca) y además lo necesita
    /// la interpolación con corrección de perspectiva.
    pub inverse_w: f32,
    pub object_position: Vector3,
    pub world_position: Vector3,
    pub object_normal: Vector3,
    pub normal: Vector3,
    pub uv: Vector2,
}

/// objeto → mundo → clip. La división de perspectiva se hace después de recortar.
fn vertex_shader(vertex: &Vertex, model: &Mat4, model_view_projection: &Mat4) -> ClipVertex {
    let p = vertex.position;

    ClipVertex {
        clip: model_view_projection.transform(Vector4::new(p.x, p.y, p.z, 1.0)),
        object_position: p,
        world_position: model.transform_point(p),
        object_normal: vertex.normal,
        // Con escala uniforme basta con transformar como dirección.
        normal: model.transform_direction(vertex.normal),
        uv: vertex.uv,
    }
}

/// División de perspectiva y paso a píxeles. Solo es válida con `w > 0`, que
/// el recorte garantiza.
fn project(vertex: &ClipVertex, width: usize, height: usize) -> ScreenVertex {
    let inverse_w = 1.0 / vertex.clip.w;
    let ndc_x = vertex.clip.x * inverse_w;
    let ndc_y = vertex.clip.y * inverse_w;

    ScreenVertex {
        x: (ndc_x + 1.0) * 0.5 * width as f32,
        // En pantalla `y` crece hacia abajo.
        y: (1.0 - ndc_y) * 0.5 * height as f32,
        inverse_w,
        object_position: vertex.object_position,
        world_position: vertex.world_position,
        object_normal: vertex.object_normal,
        normal: vertex.normal,
        uv: vertex.uv,
    }
}

/// Fragmentos y contadores de un grupo de triángulos, para juntarlos entre hilos.
#[derive(Default)]
struct RasterBatch {
    fragments: Vec<Fragment>,
    behind_camera: usize,
    clipped: usize,
    backfacing: usize,
    rasterized: usize,
}

impl RasterBatch {
    fn merge(mut self, mut other: RasterBatch) -> RasterBatch {
        self.fragments.append(&mut other.fragments);
        self.behind_camera += other.behind_camera;
        self.clipped += other.clipped;
        self.backfacing += other.backfacing;
        self.rasterized += other.rasterized;
        self
    }

    fn rasterize(&mut self, triangle: &[ClipVertex], width: usize, height: usize) {
        let screen = [0, 1, 2].map(|i| project(&triangle[i], width, height));
        match raster::rasterize(&screen, width, height, &mut self.fragments) {
            Outcome::BackFacing => self.backfacing += 1,
            Outcome::Rasterized => self.rasterized += 1,
        }
    }
}

/// Centro y radio de la esfera envolvente del objeto en el mundo. La escala es
/// uniforme, así que basta con medir cuánto se estira un eje.
pub fn world_bounds(object: &SceneObject) -> (Vector3, f32) {
    let center = object.model.transform_point(object.mesh.bounds.center);
    let radius = object.mesh.bounds.radius * object.model.transform_direction(Vector3::new(1.0, 0.0, 0.0)).length();
    (center, radius)
}

/// Dibuja un objeto. La prueba de profundidad va antes del fragment shader:
/// solo se sombrean los fragmentos que quedan al frente en ese momento, y el
/// sombreado corre en paralelo.
pub fn draw_object(framebuffer: &mut Framebuffer, object: &SceneObject, frame: &FrameContext, stats: &mut RenderStats) {
    let (width, height) = (framebuffer.width, framebuffer.height);
    let model_view_projection = frame.view_projection * object.model;

    let transformed: Vec<ClipVertex> = object
        .mesh
        .vertices
        .par_iter()
        .map(|vertex| vertex_shader(vertex, &object.model, &model_view_projection))
        .collect();

    let batch = transformed
        .par_chunks_exact(3)
        .fold(RasterBatch::default, |mut batch, triangle| {
            match clip::clip_near(triangle, frame.near) {
                Clipped::Inside => batch.rasterize(triangle, width, height),
                Clipped::Outside => batch.behind_camera += 1,
                Clipped::Split(pieces) => {
                    batch.clipped += 1;
                    for piece in &pieces {
                        batch.rasterize(piece, width, height);
                    }
                }
            }
            batch
        })
        .reduce(RasterBatch::default, RasterBatch::merge);

    // Profundidad con `1/w`: más grande = más cerca. Varía en forma lineal en
    // pantalla y su precisión no se concentra junto al plano cercano.
    let mut visible = Vec::with_capacity(batch.fragments.len());
    for fragment in &batch.fragments {
        let index = fragment.y * width + fragment.x;
        if fragment.depth > framebuffer.depths[index] {
            framebuffer.depths[index] = fragment.depth;
            visible.push((index, fragment));
        }
    }

    let to_sun = frame.light_position - object.model.origin();
    let (center, radius) = world_bounds(object);
    let to_ship = frame.ship_position - center;
    let object_context = ObjectContext {
        sun_direction: object.model.inverse_rotate_direction(to_sun),
        proximity: scene::proximity(to_ship.length(), radius),
        ship_direction: object.model.inverse_rotate_direction(to_ship),
    };

    let shaded: Vec<(usize, Vector3)> = visible
        .par_iter()
        .map(|(index, fragment)| (*index, shaders::shade(&object.material, fragment, frame, &object_context)))
        .collect();

    // En orden: si un píxel se sombreó dos veces, gana el último, que es el más cercano.
    for (index, color) in shaded {
        framebuffer.colors[index] = color;
    }

    stats.objects_drawn += 1;
    stats.triangles_submitted += object.mesh.triangle_count();
    stats.triangles_behind_camera += batch.behind_camera;
    stats.triangles_clipped += batch.clipped;
    stats.triangles_backfacing += batch.backfacing;
    stats.triangles_rasterized += batch.rasterized;
    stats.fragments_rasterized += batch.fragments.len();
    stats.fragments_shaded += visible.len();
}
