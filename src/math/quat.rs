//! Cuaterniones para orientar la nave y la cámara sin bloqueo de cardán.

use std::ops::Mul;

use raylib::math::Vector3;

/// Cuaternión `w + xi + yj + zk`. Para rotar debe ser unitario.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub const IDENTITY: Quat = Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    /// Rotación de `angle` radianes alrededor de `axis` (regla de la mano derecha).
    pub fn from_axis_angle(axis: Vector3, angle: f32) -> Quat {
        let axis = axis.normalize();
        let (s, c) = (angle * 0.5).sin_cos();
        Quat { x: axis.x * s, y: axis.y * s, z: axis.z * s, w: c }
    }

    pub fn normalize(self) -> Quat {
        let length = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        Quat { x: self.x / length, y: self.y / length, z: self.z / length, w: self.w / length }
    }

    pub fn conjugate(self) -> Quat {
        Quat { x: -self.x, y: -self.y, z: -self.z, w: self.w }
    }

    /// Aplica la rotación a un vector: `q v q*`, escrito de forma desarrollada.
    pub fn rotate(self, v: Vector3) -> Vector3 {
        let axis = Vector3::new(self.x, self.y, self.z);
        let t = axis.cross(v) * 2.0;
        v + t * self.w + axis.cross(t)
    }
}

impl Mul for Quat {
    type Output = Quat;

    /// Producto de Hamilton. `a * b` rota primero con `b` y después con `a`.
    fn mul(self, b: Quat) -> Quat {
        let a = self;
        Quat {
            w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
            x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
            y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
            z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vector3, b: Vector3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn quarter_turn_around_y() {
        let q = Quat::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), std::f32::consts::FRAC_PI_2);
        assert!(close(q.rotate(Vector3::new(1.0, 0.0, 0.0)), Vector3::new(0.0, 0.0, -1.0)));
    }

    #[test]
    fn product_composes_right_to_left() {
        let yaw = Quat::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), 0.7);
        let pitch = Quat::from_axis_angle(Vector3::new(1.0, 0.0, 0.0), -0.4);
        let v = Vector3::new(0.2, 0.5, -1.0);
        assert!(close((yaw * pitch).rotate(v), yaw.rotate(pitch.rotate(v))));
    }

    #[test]
    fn conjugate_undoes_rotation() {
        let q = Quat::from_axis_angle(Vector3::new(1.0, 1.0, 0.0), 2.1);
        let v = Vector3::new(3.0, -2.0, 1.0);
        assert!(close(q.conjugate().rotate(q.rotate(v)), v));
    }
}
