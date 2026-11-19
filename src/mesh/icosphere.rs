//! Icosfera: esfera de radio 1 generada por subdivisión, antes de rasterizar.
//!
//! Se parte de un icosaedro (20 triángulos iguales). En cada nivel, cada
//! triángulo se parte en cuatro uniendo los puntos medios de sus lados, y esos
//! puntos se empujan hasta la superficie de la esfera. Cada nivel multiplica los
//! triángulos por 4: 20, 80, 320, 1280, 5120, 20480...
//!
//! A diferencia de una esfera por latitud y longitud, los triángulos quedan
//! casi del mismo tamaño en toda la superficie, sin amontonarse en los polos.

use std::collections::HashMap;
use std::f32::consts::PI;

use raylib::math::{Vector2, Vector3};

use super::{Mesh, Vertex};

/// Triángulos de una icosfera con `level` subdivisiones.
#[cfg(test)]
pub fn triangle_count(level: u32) -> usize {
    20 * 4usize.pow(level)
}

/// Esfera de radio 1 centrada en el origen con `level` subdivisiones. La normal
/// de cada vértice es su propia posición.
pub fn icosphere(level: u32) -> Mesh {
    let (mut points, mut faces) = icosahedron();

    for _ in 0..level {
        // Un punto medio por lado. Dos triángulos vecinos comparten el lado, así
        // que se guarda para no crear el mismo punto dos veces.
        let mut midpoints: HashMap<(usize, usize), usize> = HashMap::new();
        let mut midpoint = |a: usize, b: usize, points: &mut Vec<Vector3>| {
            *midpoints.entry((a.min(b), a.max(b))).or_insert_with(|| {
                points.push(((points[a] + points[b]) * 0.5).normalize());
                points.len() - 1
            })
        };

        let mut next = Vec::with_capacity(faces.len() * 4);
        for [a, b, c] in faces {
            let ab = midpoint(a, b, &mut points);
            let bc = midpoint(b, c, &mut points);
            let ca = midpoint(c, a, &mut points);
            next.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        faces = next;
    }

    let vertex = |p: Vector3| Vertex {
        position: p,
        normal: p,
        uv: Vector2::new(p.z.atan2(p.x) / (2.0 * PI) + 0.5, p.y.asin() / PI + 0.5),
    };
    Mesh::new(faces.iter().flat_map(|face| face.map(|i| vertex(points[i]))).collect())
}

/// Los 12 vértices y 20 caras de un icosaedro inscrito en la esfera de radio 1.
/// Los vértices son las esquinas de tres rectángulos dorados perpendiculares.
fn icosahedron() -> (Vec<Vector3>, Vec<[usize; 3]>) {
    let t = (1.0 + 5f32.sqrt()) / 2.0;
    let points = [
        (-1.0, t, 0.0),
        (1.0, t, 0.0),
        (-1.0, -t, 0.0),
        (1.0, -t, 0.0),
        (0.0, -1.0, t),
        (0.0, 1.0, t),
        (0.0, -1.0, -t),
        (0.0, 1.0, -t),
        (t, 0.0, -1.0),
        (t, 0.0, 1.0),
        (-t, 0.0, -1.0),
        (-t, 0.0, 1.0),
    ]
    .map(|(x, y, z)| Vector3::new(x, y, z).normalize())
    .to_vec();

    let faces = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    (points, faces)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_level_has_four_times_the_triangles() {
        for level in 0..4 {
            assert_eq!(icosphere(level).triangle_count(), triangle_count(level));
        }
    }

    #[test]
    fn vertices_lie_on_the_unit_sphere() {
        let mesh = icosphere(3);
        assert!(mesh.vertices.iter().all(|v| (v.position.length() - 1.0).abs() < 1e-5));
        assert!(mesh.bounds.center.length() < 1e-5 && (mesh.bounds.radius - 1.0).abs() < 1e-5);
    }

    /// Antihorario visto desde afuera: la normal de cada triángulo apunta hacia
    /// afuera de la esfera. Si no, el rasterizador lo descartaría como espalda.
    #[test]
    fn triangles_face_outwards() {
        for level in 0..3 {
            let mesh = icosphere(level);
            for t in mesh.vertices.chunks_exact(3) {
                let normal = (t[1].position - t[0].position).cross(t[2].position - t[0].position);
                assert!(normal.dot(t[0].position) > 0.0, "triángulo al revés en el nivel {level}");
            }
        }
    }
}
