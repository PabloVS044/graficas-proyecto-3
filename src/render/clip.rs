//! Recorte de triángulos contra el plano cercano (Sutherland–Hodgman).
//!
//! Un vértice detrás de la cámara no se puede proyectar: al dividir entre `w`
//! cambia de lado de la pantalla. En vez de descartar el triángulo completo
//! (lo que abre agujeros cuando la cámara está pegada a una superficie), se
//! corta por el plano cercano y se dibuja solo la parte que queda al frente.
//!
//! Se recorta en espacio de clip, antes de dividir entre `w`. Ahí todo varía en
//! forma lineal, así que el punto de corte y sus atributos se obtienen
//! interpolando linealmente entre los dos vértices de la arista.
//!
//! Con la proyección de `Mat4::perspective`, `w` es la distancia frente a la
//! cámara, así que el plano cercano es `w = near`.

use raylib::math::{Vector2, Vector3, Vector4};

/// Vértice después del vertex shader y antes de la división de perspectiva.
#[derive(Clone, Copy, Debug)]
pub struct ClipVertex {
    pub clip: Vector4,
    pub object_position: Vector3,
    pub world_position: Vector3,
    pub normal: Vector3,
    pub uv: Vector2,
}

impl ClipVertex {
    /// Punto de la arista `a → b` en la fracción `t` (0 en `a`, 1 en `b`).
    fn lerp(a: &ClipVertex, b: &ClipVertex, t: f32) -> ClipVertex {
        ClipVertex {
            clip: Vector4::new(
                a.clip.x + (b.clip.x - a.clip.x) * t,
                a.clip.y + (b.clip.y - a.clip.y) * t,
                a.clip.z + (b.clip.z - a.clip.z) * t,
                a.clip.w + (b.clip.w - a.clip.w) * t,
            ),
            object_position: a.object_position + (b.object_position - a.object_position) * t,
            world_position: a.world_position + (b.world_position - a.world_position) * t,
            normal: a.normal + (b.normal - a.normal) * t,
            uv: a.uv + (b.uv - a.uv) * t,
        }
    }
}

/// Resultado de recortar un triángulo.
pub enum Clipped {
    /// Completamente al frente del plano: se dibuja tal cual.
    Inside,
    /// Completamente detrás: no se dibuja nada.
    Outside,
    /// Cruza el plano: queda un triángulo o un cuadrilátero, partido en
    /// triángulos que conservan el sentido antihorario del original.
    Split(Vec<[ClipVertex; 3]>),
}

/// Recorta el triángulo contra el plano `w = near`.
pub fn clip_near(triangle: &[ClipVertex], near: f32) -> Clipped {
    let in_front = |vertex: &ClipVertex| vertex.clip.w >= near;
    let count = triangle.iter().filter(|vertex| in_front(vertex)).count();

    match count {
        3 => return Clipped::Inside,
        0 => return Clipped::Outside,
        _ => {}
    }

    // Recorre las aristas en orden. Se queda con los vértices que están al
    // frente y agrega el punto de corte de cada arista que cruza el plano.
    let mut polygon: Vec<ClipVertex> = Vec::with_capacity(4);
    for i in 0..3 {
        let current = &triangle[i];
        let next = &triangle[(i + 1) % 3];

        if in_front(current) {
            polygon.push(*current);
        }
        if in_front(current) != in_front(next) {
            // Siempre del vértice de enfrente hacia el de atrás. Un triángulo
            // vecino recorre la misma arista en sentido contrario; así los dos
            // calculan exactamente el mismo punto y no queda una grieta entre ellos.
            let (front, back) = if in_front(current) { (current, next) } else { (next, current) };
            let t = (near - front.clip.w) / (back.clip.w - front.clip.w);
            polygon.push(ClipVertex::lerp(front, back, t));
        }
    }

    // Abanico: (0, 1, 2) y, si quedó un cuadrilátero, (0, 2, 3).
    let triangles = (1..polygon.len() - 1)
        .map(|i| [polygon[0], polygon[i], polygon[i + 1]])
        .collect();
    Clipped::Split(triangles)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEAR: f32 = 0.1;

    fn vertex(x: f32, y: f32, w: f32) -> ClipVertex {
        ClipVertex {
            clip: Vector4::new(x, y, 0.0, w),
            object_position: Vector3::new(w, 0.0, 0.0),
            world_position: Vector3::zero(),
            normal: Vector3::new(0.0, 0.0, 1.0),
            uv: Vector2::new(w, 0.0),
        }
    }

    fn split(triangle: &[ClipVertex]) -> Vec<[ClipVertex; 3]> {
        match clip_near(triangle, NEAR) {
            Clipped::Split(triangles) => triangles,
            _ => panic!("se esperaba un triángulo recortado"),
        }
    }

    #[test]
    fn keeps_triangles_in_front() {
        let triangle = [vertex(0.0, 0.0, 1.0), vertex(1.0, 0.0, 2.0), vertex(0.0, 1.0, 3.0)];
        assert!(matches!(clip_near(&triangle, NEAR), Clipped::Inside));
    }

    #[test]
    fn drops_triangles_behind() {
        let triangle = [vertex(0.0, 0.0, -1.0), vertex(1.0, 0.0, 0.05), vertex(0.0, 1.0, -3.0)];
        assert!(matches!(clip_near(&triangle, NEAR), Clipped::Outside));
    }

    #[test]
    fn one_vertex_behind_leaves_two_triangles() {
        let triangles = split(&[vertex(0.0, 0.0, -1.0), vertex(1.0, 0.0, 2.0), vertex(0.0, 1.0, 2.0)]);
        assert_eq!(triangles.len(), 2);
        assert!(triangles.iter().flatten().all(|v| v.clip.w >= NEAR - 1e-6));
    }

    #[test]
    fn two_vertices_behind_leave_one_triangle() {
        let triangles = split(&[vertex(0.0, 0.0, -1.0), vertex(1.0, 0.0, -2.0), vertex(0.0, 1.0, 2.0)]);
        assert_eq!(triangles.len(), 1);
        assert!(triangles[0].iter().all(|v| v.clip.w >= NEAR - 1e-6));
    }

    /// El punto de corte queda sobre el plano y sus atributos se interpolan
    /// con la misma fracción que la posición.
    #[test]
    fn cut_points_lie_on_the_plane_with_interpolated_attributes() {
        let triangles = split(&[vertex(0.0, 0.0, -0.9), vertex(1.0, 0.0, 1.1), vertex(0.0, 1.0, 1.1)]);
        let cut: Vec<_> = triangles.iter().flatten().filter(|v| (v.clip.w - NEAR).abs() < 1e-5).collect();

        assert!(!cut.is_empty());
        for v in cut {
            // En los datos de prueba, `uv.x` y `object_position.x` valen lo mismo que `w`.
            assert!((v.uv.x - NEAR).abs() < 1e-5);
            assert!((v.object_position.x - NEAR).abs() < 1e-5);
        }
    }

    /// Recortar no debe invertir el sentido del triángulo, o el rasterizador
    /// lo descartaría como si diera la espalda.
    #[test]
    fn clipping_preserves_winding() {
        let signed_area = |t: &[ClipVertex; 3]| {
            let p = t.map(|v| Vector2::new(v.clip.x / v.clip.w, v.clip.y / v.clip.w));
            (p[1].x - p[0].x) * (p[2].y - p[0].y) - (p[1].y - p[0].y) * (p[2].x - p[0].x)
        };
        let original = [vertex(-1.0, -1.0, 2.0), vertex(1.0, -1.0, 2.0), vertex(0.0, 1.0, -1.0)];

        for triangle in split(&original) {
            assert!(signed_area(&triangle) > 0.0);
        }
    }
}
