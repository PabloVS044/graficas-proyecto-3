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
mod exhaust;

pub use camera::ChaseCamera;
pub use controls::{PitchControl, read_input};
pub use exhaust::Exhaust;

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

/// Qué tan rápido los motores alcanzan la potencia pedida.
const THROTTLE_RESPONSE: f32 = 4.0;
/// Potencia de los motores en reposo, acelerando y con turbo (de 0 a 1).
const THROTTLE_IDLE: f32 = 0.1;
const THROTTLE_CRUISE: f32 = 0.6;

/// Modo orbital: velocidad máxima sobre la superficie, velocidad de subida y
/// bajada con la palanca a fondo y qué tan rápido se nivela la nave.
const ORBIT_MAX_SPEED: f32 = 16.0;
const CLIMB_SPEED: f32 = 6.0;
const LEVEL_RESPONSE: f32 = 3.0;

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
    /// Potencia de los motores, de 0 (reposo) a 1 (turbo). Solo es visual:
    /// la lee el shader de la nave.
    pub throttle: f32,
}

impl Ship {
    pub fn new(position: Vector3, orientation: Quat) -> Ship {
        Ship { position, velocity: Vector3::zero(), orientation, angular_velocity: Vector3::zero(), throttle: THROTTLE_IDLE }
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

    /// Cada cuerpo jala a la nave hacia su centro. Se aplica antes de `step`, y
    /// solo en vuelo libre.
    pub fn apply_gravity(&mut self, colliders: &[Collider], dt: f32) {
        self.velocity += gravity_at(self.position, colliders) * dt;
    }

    /// Avanza la nave `dt` segundos en modo orbital alrededor de `body`.
    ///
    /// La nave viaja con el planeta (su velocidad se mide respecto a él), se
    /// nivela sola para que su techo apunte hacia afuera del planeta y avanza
    /// pegada a la superficie: W/S cambian la velocidad, izquierda y derecha
    /// cambian el rumbo y el cabeceo sube o baja la altura en vez de levantar la
    /// punta. Así rodear un planeta es tan fácil como manejar.
    pub fn step_orbit(&mut self, input: &ShipInput, body: &Collider, colliders: &[Collider], dt: f32) {
        // Solo la guiñada gira la nave; el cabeceo y el alabeo los pone la nivelación.
        self.turn(&ShipInput { pitch: 0.0, roll: 0.0, ..*input }, dt);
        let up = (self.position - body.center).normalize();
        self.level(up, dt);

        let relative = self.velocity - body.velocity;
        // Rumbo sobre la superficie: la punta sin su parte hacia arriba. Mientras
        // la nave termina de nivelarse, la punta todavía puede estar inclinada.
        let forward = self.forward() - up * self.forward().dot(up);
        let forward = if forward.length() > 1e-4 { forward.normalize() } else { self.orientation.rotate(Vector3::new(0.0, 0.0, 1.0)) };
        let boost = if input.boost { BOOST } else { 1.0 };

        // Velocidad a lo largo de la superficie, hacia donde apunta la nave.
        let mut speed = relative.dot(forward).max(0.0);
        if input.thrust > 0.0 {
            speed += THRUST * boost * input.thrust * dt;
        } else if input.thrust < 0.0 {
            speed = (speed - BRAKE * -input.thrust * dt).max(0.0);
        }
        speed = (speed * (-DRAG * dt).exp()).min(ORBIT_MAX_SPEED * boost);

        let climb = input.pitch * CLIMB_SPEED;
        self.velocity = body.velocity + forward * speed + up * climb;
        self.update_throttle(input, dt);

        // Avanzar en línea recta por la tangente alejaría a la nave del planeta.
        // Después de moverla se la devuelve a su altura más lo que pidió subir,
        // así que el camino se curva alrededor del planeta.
        let distance = (self.position - body.center).length();
        self.position += self.velocity * dt;
        let center = body.center + body.velocity * dt;
        let new_up = (self.position - center).normalize();
        self.position = center + new_up * (distance + climb * dt);

        // La nave gira con la curvatura: la misma rotación que llevó la vertical
        // de antes a la de ahora. Así la punta baja sola al rodear el planeta.
        self.orientation = (rotation_between(up, new_up) * self.orientation).normalize();
        self.collide(colliders);
    }

    /// Gira la nave poco a poco para que su techo apunte hacia `up`. La rotación
    /// es la más corta: alrededor del eje perpendicular a los dos techos.
    fn level(&mut self, up: Vector3, dt: f32) {
        let blend = 1.0 - (-LEVEL_RESPONSE * dt).exp();
        let (axis, angle) = axis_angle_between(self.up(), up);
        if angle > 1e-5 {
            // Multiplicar a la izquierda aplica la rotación en ejes del mundo.
            self.orientation = (Quat::from_axis_angle(axis, angle * blend) * self.orientation).normalize();
        }
    }

    /// Avanza la nave `dt` segundos.
    pub fn step(&mut self, input: &ShipInput, colliders: &[Collider], dt: f32) {
        self.turn(input, dt);
        self.accelerate(input, dt);
        self.update_throttle(input, dt);
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

    fn update_throttle(&mut self, input: &ShipInput, dt: f32) {
        let target = match (input.thrust > 0.0, input.boost) {
            (true, true) => 1.0,
            (true, false) => THROTTLE_CRUISE,
            (false, _) => THROTTLE_IDLE,
        };
        self.throttle += (target - self.throttle) * (1.0 - (-THROTTLE_RESPONSE * dt).exp());
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

/// Eje y ángulo de la rotación más corta que lleva la dirección `from` a `to`
/// (las dos de largo 1).
fn axis_angle_between(from: Vector3, to: Vector3) -> (Vector3, f32) {
    let axis = from.cross(to);
    (axis, axis.length().atan2(from.dot(to)))
}

/// La rotación más corta que lleva la dirección `from` a `to`.
fn rotation_between(from: Vector3, to: Vector3) -> Quat {
    let (axis, angle) = axis_angle_between(from, to);
    if angle > 1e-7 { Quat::from_axis_angle(axis, angle) } else { Quat::IDENTITY }
}

/// Aceleración de la gravedad en `position`: la suma de la de cada cuerpo. En la
/// superficie vale `surface_gravity` y cae con el cuadrado de la distancia al
/// centro (`g * (R / d)²`, la ley de Newton escrita sin la constante ni la masa).
pub fn gravity_at(position: Vector3, colliders: &[Collider]) -> Vector3 {
    colliders.iter().fold(Vector3::zero(), |total, body| {
        let offset = body.center - position;
        let distance = offset.length();
        if distance == 0.0 {
            return total;
        }
        // Dentro del cuerpo no crece más; la colisión de todas formas no deja entrar.
        let falloff = body.radius / distance.max(body.radius);
        total + offset * (body.surface_gravity * falloff * falloff / distance)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;

    fn planet(center: Vector3, radius: f32, velocity: Vector3) -> Collider {
        Collider { name: "prueba", center, radius, velocity, surface_gravity: radius * crate::scene::GRAVITY_PER_RADIUS }
    }

    #[test]
    fn gravity_pulls_towards_the_planet_and_fades_with_distance() {
        let body = planet(Vector3::new(0.0, 0.0, 0.0), 6.0, Vector3::zero());
        let at_surface = gravity_at(Vector3::new(6.0, 0.0, 0.0), &[body]);
        let twice_as_far = gravity_at(Vector3::new(12.0, 0.0, 0.0), &[body]);

        assert!(at_surface.x < 0.0, "jala hacia el centro");
        assert!((at_surface.length() - body.surface_gravity).abs() < 1e-4);
        assert!((twice_as_far.length() - body.surface_gravity / 4.0).abs() < 1e-4, "al doble de distancia, la cuarta parte");
    }

    /// En modo orbital, acelerando sin tocar la altura, la nave rodea el planeta
    /// a la misma altura, se nivela y viaja con él mientras se mueve.
    #[test]
    fn orbit_mode_circles_at_constant_altitude() {
        let mut body = planet(Vector3::zero(), 6.0, Vector3::new(3.0, 0.0, 0.0));
        // A 3 de la superficie, ladeada y con la punta hacia arriba.
        let tilted = Quat::from_axis_angle(Vector3::new(1.0, 0.0, 1.0), 0.5);
        let mut ship = Ship::new(Vector3::new(0.0, 9.0, 0.0), tilted);
        let start_angle = Vector3::new(0.0, 1.0, 0.0);
        let throttle = ShipInput { thrust: 1.0, ..ShipInput::default() };

        let mut swept: f32 = 0.0;
        for _ in 0..(6.0 / DT) as usize {
            ship.step_orbit(&throttle, &body, &[body], DT);
            body.center += body.velocity * DT;
            let up = (ship.position - body.center).normalize();
            swept = swept.max(super::super::shaders::surface_distance(up, start_angle));
        }

        let altitude = (ship.position - body.center).length() - body.radius;
        assert!((altitude - 3.0).abs() < 0.5, "altura {altitude}");
        let up = (ship.position - body.center).normalize();
        assert!(ship.up().dot(up) > 0.98, "nivelada");
        assert!(swept > 1.0, "avanzó alrededor del planeta");
    }

    #[test]
    fn orbit_mode_climbs_with_pitch() {
        let body = planet(Vector3::zero(), 6.0, Vector3::zero());
        let mut ship = Ship::new(Vector3::new(0.0, 9.0, 0.0), Quat::IDENTITY);
        for _ in 0..(2.0 / DT) as usize {
            ship.step_orbit(&ShipInput { pitch: 1.0, ..ShipInput::default() }, &body, &[body], DT);
        }
        assert!(ship.position.length() > 15.0);
    }

    /// Una nave que pasa de lado sin motor se desvía hacia el planeta.
    #[test]
    fn gravity_bends_the_trajectory() {
        let body = planet(Vector3::new(10.0, 0.0, 30.0), 6.0, Vector3::zero());
        let mut ship = ship();
        ship.velocity = Vector3::new(0.0, 0.0, 20.0);
        for _ in 0..(2.0 / DT) as usize {
            ship.apply_gravity(&[body], DT);
            ship.step(&ShipInput::default(), &[body], DT);
        }
        assert!(ship.velocity.x > 1.0, "la velocidad gira hacia el planeta");
    }

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
    fn engines_glow_more_with_boost() {
        let mut ship = ship();
        run(&mut ship, FORWARD, 2.0);
        let cruising = ship.throttle;
        run(&mut ship, ShipInput { boost: true, ..FORWARD }, 2.0);
        assert!(cruising > THROTTLE_IDLE && ship.throttle > cruising);
        run(&mut ship, ShipInput::default(), 3.0);
        assert!((ship.throttle - THROTTLE_IDLE).abs() < 0.01);
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
        let planet = planet(Vector3::new(0.0, 0.0, 30.0), 10.0, Vector3::zero());
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
        let planet = planet(Vector3::new(0.0, 0.0, 10.0), 9.5, Vector3::new(0.0, 0.0, -3.0));
        let mut ship = ship();
        ship.step(&ShipInput::default(), &[planet], DT);
        assert!(ship.velocity.z <= -3.0 + 1e-3);
    }
}
