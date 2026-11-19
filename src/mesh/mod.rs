//! Geometría: vértices, mallas y su esfera envolvente.

pub mod icosphere;
mod obj;
pub mod shapes;

pub use icosphere::icosphere;
pub use obj::{Normals, Obj};

use raylib::math::{Vector2, Vector3};

/// Vértice en espacio del objeto, tal como lo produce el cargador o un generador.
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Vector3,
    pub normal: Vector3,
    pub uv: Vector2,
}

/// Esfera que contiene a toda la malla. La usan el culling, la colisión y la
/// elección de nivel de detalle.
#[derive(Clone, Copy, Debug)]
pub struct BoundingSphere {
    pub center: Vector3,
    pub radius: f32,
}

/// Malla de triángulos sin índices: cada tres vértices seguidos forman un
/// triángulo, antihorario visto desde afuera.
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub bounds: BoundingSphere,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>) -> Mesh {
        assert!(vertices.len().is_multiple_of(3), "una malla necesita tres vértices por triángulo");
        let bounds = bounding_sphere(&vertices);
        Mesh { vertices, bounds }
    }

    pub fn triangle_count(&self) -> usize {
        self.vertices.len() / 3
    }
}

/// Centro de la caja que contiene a los vértices y el radio hasta el más lejano.
/// No es la esfera mínima, pero es simple y siempre los contiene.
fn bounding_sphere(vertices: &[Vertex]) -> BoundingSphere {
    let Some(first) = vertices.first() else {
        return BoundingSphere { center: Vector3::zero(), radius: 0.0 };
    };

    let (mut min, mut max) = (first.position, first.position);
    for vertex in vertices {
        let p = vertex.position;
        min = Vector3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z));
        max = Vector3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z));
    }

    let center = (min + max) * 0.5;
    let radius = vertices
        .iter()
        .map(|vertex| (vertex.position - center).length())
        .fold(0.0, f32::max);

    BoundingSphere { center, radius }
}
