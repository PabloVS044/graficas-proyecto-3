//! Cámara en tercera persona: detrás y un poco arriba de la nave.

use raylib::math::Vector3;

use super::Ship;
use crate::render::Camera;
use crate::scene::Collider;

/// Distancia detrás de la nave.
const DISTANCE: f32 = 7.0;
/// Altura sobre la nave, en la dirección de su techo.
const HEIGHT: f32 = 2.2;
/// Qué tan adelante de la nave apunta la cámara.
const LOOK_AHEAD: f32 = 6.0;
/// Qué tan rápido alcanza su lugar. Más bajo deja más retraso al girar y
/// acelerar, lo que hace notar el movimiento.
const FOLLOW: f32 = 6.0;
/// Qué tan rápido se inclina con la nave al alabear.
const TILT_FOLLOW: f32 = 4.0;
/// Separación mínima con la superficie de un cuerpo, para no meterse en él.
const SURFACE_MARGIN: f32 = 0.6;

#[derive(Clone, Copy, Debug)]
pub struct ChaseCamera {
    pub position: Vector3,
    up: Vector3,
}

impl ChaseCamera {
    /// Empieza ya en su lugar, sin retraso.
    pub fn behind(ship: &Ship) -> ChaseCamera {
        ChaseCamera { position: desired_position(ship), up: ship.up() }
    }

    pub fn step(&mut self, ship: &Ship, colliders: &[Collider], dt: f32) {
        let follow = 1.0 - (-FOLLOW * dt).exp();
        self.position += (desired_position(ship) - self.position) * follow;

        let tilt = 1.0 - (-TILT_FOLLOW * dt).exp();
        self.up = (self.up + (ship.up() - self.up) * tilt).normalize();

        // Si la nave pasa rozando un planeta, la cámara de atrás podría
        // quedar dentro de él.
        for collider in colliders {
            let offset = self.position - collider.center;
            let distance = offset.length();
            let minimum = collider.radius + SURFACE_MARGIN;
            if distance < minimum && distance > 0.0 {
                self.position = collider.center + offset * (minimum / distance);
            }
        }
    }

    pub fn camera(&self, ship: &Ship, field_of_view: f32, near: f32, far: f32) -> Camera {
        Camera {
            position: self.position,
            target: ship.position + ship.forward() * LOOK_AHEAD,
            up: self.up,
            field_of_view,
            near,
            far,
        }
    }
}

fn desired_position(ship: &Ship) -> Vector3 {
    ship.position - ship.forward() * DISTANCE + ship.up() * HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Quat;

    #[test]
    fn settles_behind_and_above_the_ship() {
        let ship = Ship::new(Vector3::new(5.0, 0.0, 0.0), Quat::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), 1.0));
        let mut camera = ChaseCamera { position: Vector3::zero(), up: Vector3::new(0.0, 1.0, 0.0) };
        for _ in 0..600 {
            camera.step(&ship, &[], 1.0 / 120.0);
        }

        let offset = camera.position - ship.position;
        assert!(offset.dot(ship.forward()) < -DISTANCE + 0.01);
        assert!((offset.dot(ship.up()) - HEIGHT).abs() < 0.01);
    }

    #[test]
    fn stays_outside_planets() {
        // La nave sube en vertical justo sobre la superficie: su lugar "detrás"
        // queda dentro del planeta.
        let nose_up = Quat::from_axis_angle(Vector3::new(1.0, 0.0, 0.0), -std::f32::consts::FRAC_PI_2);
        let ship = Ship::new(Vector3::new(0.0, 11.5, 0.0), nose_up);
        let planet = Collider { center: Vector3::zero(), radius: 10.0, velocity: Vector3::zero() };
        assert!((desired_position(&ship) - planet.center).length() < planet.radius);
        let mut camera = ChaseCamera::behind(&ship);
        for _ in 0..120 {
            camera.step(&ship, &[planet], 1.0 / 120.0);
            assert!((camera.position - planet.center).length() >= planet.radius + SURFACE_MARGIN - 1e-3);
        }
    }
}
