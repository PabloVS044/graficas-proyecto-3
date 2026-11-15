//! Las etapas que recorre cada objeto: vertex shader, rasterización, prueba
//! de profundidad y fragment shader.

use rayon::prelude::*;
use raylib::math::{Vector2, Vector3, Vector4};

use super::fragment::Fragment;
use super::raster::{self, Outcome};
use super::{FrameContext, Framebuffer, RenderStats};
use crate::math::Mat4;
use crate::mesh::Vertex;
use crate::scene::SceneObject;
use crate::shaders;

/// Lo que devuelve el vertex shader: el vértice en pantalla, junto con lo que
/// el rasterizador interpola para cada fragmento.
#[derive(Clone, Copy, Debug)]
pub struct TransformedVertex {
    /// `x`, `y` en píxeles; `z` es la profundidad.
    pub screen: Vector3,
    /// `1 / w`: lo necesita la interpolación con corrección de perspectiva.
    pub inverse_w: f32,
    pub object_position: Vector3,
    pub world_position: Vector3,
    pub normal: Vector3,
    pub uv: Vector2,
}

/// objeto → mundo → clip → (división de perspectiva) → pantalla.
fn vertex_shader(vertex: &Vertex, model: &Mat4, model_view_projection: &Mat4, width: usize, height: usize) -> TransformedVertex {
    let p = vertex.position;
    let clip = model_view_projection.transform(Vector4::new(p.x, p.y, p.z, 1.0));

    let inverse_w = 1.0 / clip.w;
    let ndc = Vector3::new(clip.x * inverse_w, clip.y * inverse_w, clip.z * inverse_w);

    TransformedVertex {
        screen: Vector3::new(
            (ndc.x + 1.0) * 0.5 * width as f32,
            // En pantalla `y` crece hacia abajo.
            (1.0 - ndc.y) * 0.5 * height as f32,
            ndc.z,
        ),
        inverse_w,
        object_position: p,
        world_position: model.transform_point(p),
        // Con escala uniforme basta con transformar como dirección.
        normal: model.transform_direction(vertex.normal),
        uv: vertex.uv,
    }
}

/// Fragmentos y contadores de un grupo de triángulos, para juntarlos entre hilos.
#[derive(Default)]
struct RasterBatch {
    fragments: Vec<Fragment>,
    behind_camera: usize,
    backfacing: usize,
    rasterized: usize,
}

impl RasterBatch {
    fn merge(mut self, mut other: RasterBatch) -> RasterBatch {
        self.fragments.append(&mut other.fragments);
        self.behind_camera += other.behind_camera;
        self.backfacing += other.backfacing;
        self.rasterized += other.rasterized;
        self
    }
}

/// Dibuja un objeto. La prueba de profundidad va antes del fragment shader:
/// solo se sombrean los fragmentos que quedan al frente en ese momento, y el
/// sombreado corre en paralelo.
pub fn draw_object(framebuffer: &mut Framebuffer, object: &SceneObject, frame: &FrameContext, stats: &mut RenderStats) {
    let (width, height) = (framebuffer.width, framebuffer.height);
    let model_view_projection = frame.view_projection * object.model;

    let transformed: Vec<TransformedVertex> = object
        .mesh
        .vertices
        .par_iter()
        .map(|vertex| vertex_shader(vertex, &object.model, &model_view_projection, width, height))
        .collect();

    let batch = transformed
        .par_chunks_exact(3)
        .fold(RasterBatch::default, |mut batch, triangle| {
            match raster::rasterize(triangle, width, height, &mut batch.fragments) {
                Outcome::BehindCamera => batch.behind_camera += 1,
                Outcome::BackFacing => batch.backfacing += 1,
                Outcome::Rasterized => batch.rasterized += 1,
            }
            batch
        })
        .reduce(RasterBatch::default, RasterBatch::merge);

    let mut visible = Vec::with_capacity(batch.fragments.len());
    for fragment in &batch.fragments {
        let index = fragment.y * width + fragment.x;
        if fragment.depth < framebuffer.depths[index] {
            framebuffer.depths[index] = fragment.depth;
            visible.push((index, fragment));
        }
    }

    let shaded: Vec<(usize, Vector3)> = visible
        .par_iter()
        .map(|(index, fragment)| (*index, shaders::shade(&object.material, fragment, frame)))
        .collect();

    // En orden: si un píxel se sombreó dos veces, gana el último, que es el más cercano.
    for (index, color) in shaded {
        framebuffer.colors[index] = color;
    }

    stats.objects_drawn += 1;
    stats.triangles_submitted += object.mesh.triangle_count();
    stats.triangles_behind_camera += batch.behind_camera;
    stats.triangles_backfacing += batch.backfacing;
    stats.triangles_rasterized += batch.rasterized;
    stats.fragments_rasterized += batch.fragments.len();
    stats.fragments_shaded += visible.len();
}
