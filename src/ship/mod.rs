//! La nave: estado, física de vuelo y colisión.
//!
//! La física avanza en pasos fijos (`Simulation`), así que la nave responde
//! igual en una computadora que dibuja a 30 cuadros por segundo que en una
//! que dibuja a 144.
//!
//! Ejes locales del modelo: la punta mira hacia `+z`, el techo hacia `+y`, y
//! el ala derecha (vista desde atrás) queda en `-x`.

mod camera;
mod controls;

pub use camera::ChaseCamera;
pub use controls::{PitchControl, read_input};

use raylib::math::Vector3;

use crate::math::Quat;
use crate::scene::Collider;

/// Aceleración del motor, en unidades por segundo².
const THRUST: f32 = 18.0;
/// El turbo multiplica el empuje y la velocidad máxima.
const BOOST: f32 = 2.5;
/// Desaceleración del freno, en unidades por segundo².
const BRAKE: f32 = 30.0;
const MAX_SPEED: f32 = 35.0;
/// Pérdida suave de velocidad sin motor, por segundo. No es realista en el
/// espacio, pero evita que la nave se aleje para siempre al soltar el motor.
const DRAG: f32 = 0.15;

/// Radianes por segundo con la palanca a fondo: cabeceo, guiñada y alabeo.
const PITCH_RATE: f32 = 1.4;
const YAW_RATE: f32 = 1.1;
const ROLL_RATE: f32 = 2.2;
/// Qué tan rápido el giro alcanza la velocidad pedida. Más alto se siente más
/// rígido; más bajo, más pesado.
const TURN_RESPONSE: f32 = 5.0;

/// Radio de la esfera que representa a la nave en las colisiones.
pub const COLLISION_RADIUS: f32 = 1.2;

/// Lo que pide quien juega, con valores entre -1 y 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShipInput {
    /// 1 acelera, -1 frena.
    pub thrust: f32,
    /// 1 levanta la punta.
    pub pitch: f32,
    /// 1 gira hacia la derecha.
    pub yaw: f32,
    /// 1 baja el ala derecha.
    pub roll: f32,
    pub boost: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Ship {
    pub position: Vector3,
    pub velocity: Vector3,
    pub orientation: Quat,
    /// Radianes por segundo alrededor de los ejes locales `x`, `y`, `z`.
    pub angular_velocity: Vector3,
}

impl Ship {
    pub fn new(position: Vector3, orientation: Quat) -> Ship {
        Ship { position, velocity: Vector3::zero(), orientation, angular_velocity: Vector3::zero() }
    }

    pub fn forward(&self) -> Vector3 {
        self.orientation.rotate(Vector3::new(0.0, 0.0, 1.0))
    }

    pub fn up(&self) -> Vector3 {
        self.orientation.rotate(Vector3::new(0.0, 1.0, 0.0))
    }

    pub fn speed(&self) -> f32 {
        self.velocity.length()
    }

    /// Avanza la nave `dt` segundos.
    pub fn step(&mut self, input: &ShipInput, colliders: &[Collider], dt: f32) {
        self.turn(input, dt);
        self.accelerate(input, dt);
        self.position += self.velocity * dt;
        self.collide(colliders);
    }

    /// El giro no cambia de golpe: la velocidad angular se acerca a la pedida
    /// de forma exponencial, y con ella se rota la orientación.
    fn turn(&mut self, input: &ShipInput, dt: f32) {
        // Signos según los ejes locales: levantar la punta es girar en `-x`,
        // girar a la derecha es girar en `-y`, bajar el ala derecha es `+z`.
        let target = Vector3::new(-input.pitch * PITCH_RATE, -input.yaw * YAW_RATE, input.roll * ROLL_RATE);
        let blend = 1.0 - (-TURN_RESPONSE * dt).exp();
        self.angular_velocity += (target - self.angular_velocity) * blend;

        let angle = self.angular_velocity.length() * dt;
        if angle > 0.0 {
            // Multiplicar a la derecha aplica la rotación en ejes locales.
            let delta = Quat::from_axis_angle(self.angular_velocity, angle);
            self.orientation = (self.orientation * delta).normalize();
        }
    }

    fn accelerate(&mut self, input: &ShipInput, dt: f32) {
        let boost = if input.boost { BOOST } else { 1.0 };

        if input.thrust > 0.0 {
            self.velocity += self.forward() * (THRUST * boost * input.thrust * dt);
        } else if input.thrust < 0.0 {
            // Frena hasta detenerse, sin echarse en reversa.
            let speed = self.speed();
            let reduction = (BRAKE * -input.thrust * dt).min(speed);
            if speed > 0.0 {
                self.velocity -= self.velocity * (reduction / speed);
            }
        }

        self.velocity *= (-DRAG * dt).exp();

        let max_speed = MAX_SPEED * boost;
        let speed = self.speed();
        if speed > max_speed {
            self.velocity *= max_speed / speed;
        }
    }

    /// Si la nave entra en un cuerpo, se empuja a su superficie y pierde la
    /// parte de la velocidad que apuntaba hacia adentro. La velocidad se mide
    /// respecto al cuerpo, porque el planeta también se mueve en su órbita.
    fn collide(&mut self, colliders: &[Collider]) {
        for collider in colliders {
            let offset = self.position - collider.center;
            let distance = offset.length();
            let minimum = collider.radius + COLLISION_RADIUS;
            if distance >= minimum || distance == 0.0 {
                continue;
            }

            let normal = offset * (1.0 / distance);
            self.position = collider.center + normal * minimum;

            let relative = self.velocity - collider.velocity;
            let inward = relative.dot(normal);
            if inward < 0.0 {
                self.velocity = collider.velocity + relative - normal * inward;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;

    fn ship() -> Ship {
        Ship::new(Vector3::zero(), Quat::IDENTITY)
    }

    fn run(ship: &mut Ship, input: ShipInput, seconds: f32) {
        for _ in 0..(seconds / DT).round() as usize {
            ship.step(&input, &[], DT);
        }
    }

    const FORWARD: ShipInput = ShipInput { thrust: 1.0, pitch: 0.0, yaw: 0.0, roll: 0.0, boost: false };

    #[test]
    fn speeds_up_gradually_until_max_speed() {
        let mut ship = ship();
        run(&mut ship, FORWARD, 0.2);
        let early = ship.speed();
        run(&mut ship, FORWARD, 30.0);

        assert!(early > 0.0 && early < 5.0);
        assert!((ship.speed() - MAX_SPEED).abs() < 0.5);
        assert!(ship.velocity.z > 0.0, "avanza hacia la punta");
    }

    #[test]
    fn brakes_to_a_stop_without_reversing() {
        let mut ship = ship();
        run(&mut ship, FORWARD, 3.0);
        run(&mut ship, ShipInput { thrust: -1.0, ..ShipInput::default() }, 5.0);
        assert_eq!(ship.speed(), 0.0);
    }

    #[test]
    fn turning_ramps_up_and_points_the_nose() {
        let mut ship = ship();
        let right = ShipInput { yaw: 1.0, ..ShipInput::default() };
        ship.step(&right, &[], DT);
        assert!(ship.angular_velocity.length() < YAW_RATE * 0.1, "el giro empieza suave");

        run(&mut ship, right, 0.5);
        // Visto desde atrás la derecha es `-x`.
        assert!(ship.forward().x < -0.2);

        let mut ship = self::ship();
        run(&mut ship, ShipInput { pitch: 1.0, ..ShipInput::default() }, 0.5);
        assert!(ship.forward().y > 0.2, "levanta la punta");

        let mut ship = self::ship();
        run(&mut ship, ShipInput { roll: 1.0, ..ShipInput::default() }, 0.3);
        let right_wing = ship.orientation.rotate(Vector3::new(-1.0, 0.0, 0.0));
        assert!(right_wing.y < -0.2, "baja el ala derecha");
    }

    #[test]
    fn stops_turning_when_released() {
        let mut ship = ship();
        run(&mut ship, ShipInput { roll: 1.0, ..ShipInput::default() }, 1.0);
        run(&mut ship, ShipInput::default(), 3.0);
        assert!(ship.angular_velocity.length() < 1e-3);
    }

    #[test]
    fn cannot_go_through_a_planet() {
        let planet = Collider { center: Vector3::new(0.0, 0.0, 30.0), radius: 10.0, velocity: Vector3::zero() };
        let mut ship = ship();
        for _ in 0..(10.0 / DT) as usize {
            ship.step(&FORWARD, &[planet], DT);
            let distance = (ship.position - planet.center).length();
            assert!(distance >= planet.radius + COLLISION_RADIUS - 1e-3);
        }
        assert!(ship.velocity.z.abs() < 1e-3, "la velocidad hacia el planeta se anula");
    }

    /// Un planeta que viene hacia la nave la empuja con él.
    #[test]
    fn moving_planet_pushes_the_ship() {
        let planet = Collider { center: Vector3::new(0.0, 0.0, 10.0), radius: 9.5, velocity: Vector3::new(0.0, 0.0, -3.0) };
        let mut ship = ship();
        ship.step(&ShipInput::default(), &[planet], DT);
        assert!(ship.velocity.z <= -3.0 + 1e-3);
    }
}
