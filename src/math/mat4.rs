//! Matriz 4x4 para transformaciones en coordenadas homogéneas.
//!
//! Convenciones (las mismas de OpenGL y de lo visto en clase):
//!
//! - Vectores columna: una matriz transforma con `M * v`. Al componer, la que
//!   está más a la derecha se aplica primero: `T * R * S` escala, luego rota y
//!   al final traslada.
//! - Sistema de mano derecha: `x` a la derecha, `y` arriba, y la cámara mira
//!   hacia `-z`.
//! - En espacio de clip, lo visible cumple `-w <= x, y, z <= w`.

use std::ops::Mul;

use raylib::math::{Vector3, Vector4};

use super::Quat;

/// Guardada por columnas: `m[columna * 4 + fila]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    pub m: [f32; 16],
}

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4::from_rows([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    /// Construye la matriz escribiéndola como se lee en papel, fila por fila.
    pub const fn from_rows(rows: [[f32; 4]; 4]) -> Mat4 {
        let mut m = [0.0; 16];
        let mut row = 0;
        while row < 4 {
            let mut column = 0;
            while column < 4 {
                m[column * 4 + row] = rows[row][column];
                column += 1;
            }
            row += 1;
        }
        Mat4 { m }
    }

    #[inline]
    pub fn at(&self, row: usize, column: usize) -> f32 {
        self.m[column * 4 + row]
    }

    pub fn translation(offset: Vector3) -> Mat4 {
        Mat4::from_rows([
            [1.0, 0.0, 0.0, offset.x],
            [0.0, 1.0, 0.0, offset.y],
            [0.0, 0.0, 1.0, offset.z],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Escala igual en los tres ejes. Con escala uniforme, las normales se
    /// pueden transformar con la misma matriz y solo hay que normalizarlas.
    pub fn scale(factor: f32) -> Mat4 {
        Mat4::from_rows([
            [factor, 0.0, 0.0, 0.0],
            [0.0, factor, 0.0, 0.0],
            [0.0, 0.0, factor, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotation_x(angle: f32) -> Mat4 {
        let (s, c) = angle.sin_cos();
        Mat4::from_rows([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, c, -s, 0.0],
            [0.0, s, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotation_y(angle: f32) -> Mat4 {
        let (s, c) = angle.sin_cos();
        Mat4::from_rows([
            [c, 0.0, s, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [-s, 0.0, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn rotation_z(angle: f32) -> Mat4 {
        let (s, c) = angle.sin_cos();
        Mat4::from_rows([
            [c, -s, 0.0, 0.0],
            [s, c, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Rotación equivalente a un cuaternión unitario.
    pub fn rotation(q: Quat) -> Mat4 {
        let Quat { x, y, z, w } = q;
        Mat4::from_rows([
            [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - w * z), 2.0 * (x * z + w * y), 0.0],
            [2.0 * (x * y + w * z), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - w * x), 0.0],
            [2.0 * (x * z - w * y), 2.0 * (y * z + w * x), 1.0 - 2.0 * (x * x + y * y), 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Matriz de vista: lleva el mundo al espacio de la cámara, donde la
    /// cámara está en el origen y mira hacia `-z`.
    ///
    /// Sus filas son los ejes de la cámara (derecha, arriba, atrás) expresados
    /// en el mundo; la última columna traslada el ojo al origen.
    pub fn look_at(eye: Vector3, target: Vector3, up: Vector3) -> Mat4 {
        let forward = (target - eye).normalize();
        let right = forward.cross(up).normalize();
        let up = right.cross(forward);

        Mat4::from_rows([
            [right.x, right.y, right.z, -right.dot(eye)],
            [up.x, up.y, up.z, -up.dot(eye)],
            [-forward.x, -forward.y, -forward.z, forward.dot(eye)],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Proyección en perspectiva. `field_of_view` es vertical, en radianes.
    ///
    /// Después de multiplicar, `w = -z_vista` (la distancia frente a la cámara),
    /// y al dividir entre `w` lo lejano se acerca al centro de la pantalla. El
    /// plano cercano queda en `z_ndc = -1` y el lejano en `z_ndc = 1`.
    pub fn perspective(field_of_view: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
        let f = 1.0 / (field_of_view * 0.5).tan();
        Mat4::from_rows([
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, (far + near) / (near - far), 2.0 * far * near / (near - far)],
            [0.0, 0.0, -1.0, 0.0],
        ])
    }

    pub fn transform(&self, v: Vector4) -> Vector4 {
        let m = &self.m;
        Vector4::new(
            m[0] * v.x + m[4] * v.y + m[8] * v.z + m[12] * v.w,
            m[1] * v.x + m[5] * v.y + m[9] * v.z + m[13] * v.w,
            m[2] * v.x + m[6] * v.y + m[10] * v.z + m[14] * v.w,
            m[3] * v.x + m[7] * v.y + m[11] * v.z + m[15] * v.w,
        )
    }

    /// Transforma un punto (`w = 1`): le afecta la traslación.
    pub fn transform_point(&self, p: Vector3) -> Vector3 {
        let v = self.transform(Vector4::new(p.x, p.y, p.z, 1.0));
        Vector3::new(v.x, v.y, v.z)
    }

    /// Transforma una dirección (`w = 0`): la traslación no le afecta.
    pub fn transform_direction(&self, d: Vector3) -> Vector3 {
        let v = self.transform(Vector4::new(d.x, d.y, d.z, 0.0));
        Vector3::new(v.x, v.y, v.z)
    }
}

impl Mul for Mat4 {
    type Output = Mat4;

    /// `self * other`: primero se aplica `other` y después `self`.
    fn mul(self, other: Mat4) -> Mat4 {
        let mut m = [0.0; 16];
        for column in 0..4 {
            for row in 0..4 {
                m[column * 4 + row] = (0..4).map(|k| self.at(row, k) * other.at(k, column)).sum();
            }
        }
        Mat4 { m }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vector3, b: Vector3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn composition_applies_right_to_left() {
        // Escalar y después trasladar: (1, 0, 0) -> (2, 0, 0) -> (2, 5, 0).
        let model = Mat4::translation(Vector3::new(0.0, 5.0, 0.0)) * Mat4::scale(2.0);
        let p = model.transform_point(Vector3::new(1.0, 0.0, 0.0));
        assert!(close(p, Vector3::new(2.0, 5.0, 0.0)));
    }

    #[test]
    fn rotation_y_turns_x_towards_minus_z() {
        let p = Mat4::rotation_y(std::f32::consts::FRAC_PI_2).transform_point(Vector3::new(1.0, 0.0, 0.0));
        assert!(close(p, Vector3::new(0.0, 0.0, -1.0)));
    }

    #[test]
    fn directions_ignore_translation() {
        let m = Mat4::translation(Vector3::new(3.0, 4.0, 5.0));
        assert!(close(m.transform_direction(Vector3::new(0.0, 1.0, 0.0)), Vector3::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn look_at_puts_eye_at_origin_and_target_in_front() {
        let eye = Vector3::new(3.0, 2.0, 10.0);
        let target = Vector3::new(-1.0, 0.5, 2.0);
        let view = Mat4::look_at(eye, target, Vector3::new(0.0, 1.0, 0.0));

        assert!(close(view.transform_point(eye), Vector3::zero()));
        let t = view.transform_point(target);
        let distance = (target - eye).length();
        assert!(close(t, Vector3::new(0.0, 0.0, -distance)));
    }

    #[test]
    fn perspective_maps_near_and_far_planes() {
        let projection = Mat4::perspective(1.0, 16.0 / 9.0, 0.1, 100.0);
        let ndc_z = |z: f32| {
            let clip = projection.transform(Vector4::new(0.0, 0.0, z, 1.0));
            (clip.z / clip.w, clip.w)
        };

        let (near, w_near) = ndc_z(-0.1);
        let (far, w_far) = ndc_z(-100.0);
        assert!((near + 1.0).abs() < 1e-4);
        assert!((far - 1.0).abs() < 1e-4);
        // `w` es la distancia frente a la cámara.
        assert!((w_near - 0.1).abs() < 1e-6 && (w_far - 100.0).abs() < 1e-3);
    }

    #[test]
    fn quaternion_matrix_matches_quaternion_rotation() {
        let q = Quat::from_axis_angle(Vector3::new(1.0, 2.0, -0.5), 1.3);
        let v = Vector3::new(0.3, -1.0, 2.0);
        assert!(close(Mat4::rotation(q).transform_point(v), q.rotate(v)));
    }
}
