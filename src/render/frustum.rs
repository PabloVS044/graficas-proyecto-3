//! Frustum: la pirámide truncada que ve la cámara. Sirve para descartar un
//! objeto completo antes de mandarlo al pipeline, si su esfera envolvente queda
//! fuera de la vista.

use raylib::math::{Vector3, Vector4};

use crate::math::Mat4;

/// Los seis planos del frustum, con la normal apuntando hacia adentro. Un punto
/// `p` está del lado de adentro de un plano `(n, d)` si `n · p + d >= 0`.
pub struct Frustum {
    planes: [Vector4; 6],
}

impl Frustum {
    /// Saca los planos de la matriz `projection * view` (método de Gribb y
    /// Hartmann). Un punto está dentro de la vista si, en coordenadas de clip,
    /// `-w <= x <= w`, y lo mismo para `y` y `z`. Como `x` y `w` son las filas 0
    /// y 3 de la matriz por el punto, `w + x >= 0` es el plano izquierdo con
    /// coeficientes `fila3 + fila0`, `w - x >= 0` el derecho, etc.
    pub fn from_view_projection(m: &Mat4) -> Frustum {
        let row = |r: usize| Vector4::new(m.at(r, 0), m.at(r, 1), m.at(r, 2), m.at(r, 3));
        let add = |a: Vector4, b: Vector4| Vector4::new(a.x + b.x, a.y + b.y, a.z + b.z, a.w + b.w);
        let sub = |a: Vector4, b: Vector4| Vector4::new(a.x - b.x, a.y - b.y, a.z - b.z, a.w - b.w);
        let w = row(3);

        let planes = [
            add(w, row(0)), // izquierdo
            sub(w, row(0)), // derecho
            add(w, row(1)), // abajo
            sub(w, row(1)), // arriba
            add(w, row(2)), // cercano
            sub(w, row(2)), // lejano
        ];
        // Normalizados, `n · p + d` es la distancia al plano en unidades del mundo.
        Frustum {
            planes: planes.map(|p| {
                let length = Vector3::new(p.x, p.y, p.z).length();
                Vector4::new(p.x / length, p.y / length, p.z / length, p.w / length)
            }),
        }
    }

    /// Si alguna parte de la esfera puede quedar a la vista. Es conservador: una
    /// esfera cerca de una esquina, fuera de la vista pero no detrás de un solo
    /// plano, se dibuja igual; lo que se descarta seguro no se ve.
    pub fn contains_sphere(&self, center: Vector3, radius: f32) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.x * center.x + plane.y * center.y + plane.z * center.z + plane.w >= -radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Camera;

    fn frustum() -> Frustum {
        // En el origen mirando hacia `-z`, como la cámara de la vista.
        let camera = Camera {
            position: Vector3::zero(),
            target: Vector3::new(0.0, 0.0, -1.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            field_of_view: 60f32.to_radians(),
            near: 0.1,
            far: 100.0,
        };
        Frustum::from_view_projection(&(camera.projection(16.0 / 9.0) * camera.view()))
    }

    #[test]
    fn sphere_in_front_is_visible() {
        assert!(frustum().contains_sphere(Vector3::new(0.0, 0.0, -10.0), 1.0));
    }

    #[test]
    fn spheres_behind_beside_or_beyond_are_culled() {
        let frustum = frustum();
        assert!(!frustum.contains_sphere(Vector3::new(0.0, 0.0, 10.0), 1.0), "detrás");
        assert!(!frustum.contains_sphere(Vector3::new(50.0, 0.0, -10.0), 1.0), "a un lado");
        assert!(!frustum.contains_sphere(Vector3::new(0.0, 0.0, -150.0), 1.0), "más allá del plano lejano");
    }

    #[test]
    fn sphere_crossing_the_edge_is_kept() {
        // El centro queda fuera del borde derecho, pero el radio lo alcanza.
        let frustum = frustum();
        let half_width = (30f32.to_radians()).tan() * 10.0 * 16.0 / 9.0;
        assert!(frustum.contains_sphere(Vector3::new(half_width + 1.0, 0.0, -10.0), 2.0));
    }
}
