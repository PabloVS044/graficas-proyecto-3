//! Avance del mundo en pasos fijos.
//!
//! Cada cuadro dura lo que tarde la computadora en dibujarlo. Si la física
//! avanzara con ese tiempo variable, la nave respondería distinto en cada
//! equipo. En cambio, el tiempo real se acumula y se consume en pasos de
//! duración fija: en una computadora lenta se dan varios pasos por cuadro, y
//! en una rápida a veces ninguno, pero el resultado es el mismo.

use crate::scene::{SolarSystem, proximity};
use crate::scene::planets::PLANETS;
use crate::ship::{ChaseCamera, Exhaust, Ship, ShipInput};

/// Duración de un paso de física, en segundos.
pub const STEP: f32 = 1.0 / 120.0;
/// Un cuadro muy lento (por ejemplo al mover la ventana) no debe hacer saltar
/// la simulación: se avanza como máximo este tiempo por cuadro.
const MAX_FRAME_TIME: f32 = 0.1;

pub struct Simulation {
    /// Segundos de simulación. Todo lo animado depende de esto.
    pub time: f32,
    pub ship: Ship,
    pub camera: ChaseCamera,
    pub paused: bool,
    /// Un lugar por planeta de `PLANETS`: si la nave ya llegó cerca de él.
    pub discovered: Vec<bool>,
    /// Si los cuerpos atraen a la nave.
    pub gravity: bool,
    /// Cuerpo que la nave está orbitando en modo orbital: su índice en
    /// `SolarSystem::colliders` y su nombre. `None` es vuelo libre.
    pub orbit: Option<(usize, &'static str)>,
    /// Partículas que suelta el turbo.
    pub exhaust: Exhaust,
    accumulator: f32,
}

/// Qué tan cerca hay que llegar a un planeta para contarlo como descubierto.
const DISCOVERY_PROXIMITY: f32 = 0.6;
/// Qué tan cerca hay que estar de un cuerpo para entrar en modo orbital.
const ORBIT_ENGAGE_PROXIMITY: f32 = 0.3;
/// Si en modo orbital la nave sube hasta quedar así de lejos, vuelve al vuelo libre.
const ORBIT_RELEASE_PROXIMITY: f32 = 0.02;

impl Simulation {
    pub fn new(scene: &SolarSystem) -> Simulation {
        let ship = scene.ship_start();
        Simulation {
            time: 0.0,
            ship,
            camera: ChaseCamera::behind(&ship),
            paused: false,
            discovered: vec![false; PLANETS.len()],
            gravity: true,
            orbit: None,
            exhaust: Exhaust::new(),
            accumulator: 0.0,
        }
    }

    /// Vuelve la nave al inicio sin detener las órbitas.
    pub fn reset_ship(&mut self, scene: &SolarSystem) {
        self.ship = scene.ship_start();
        self.camera = ChaseCamera::behind(&self.ship);
        self.orbit = None;
        self.exhaust.clear();
    }

    /// El cuerpo más cercano que se puede orbitar desde donde está la nave.
    pub fn orbit_candidate(&self, scene: &SolarSystem) -> Option<(usize, &'static str)> {
        scene
            .colliders(self.time)
            .iter()
            .enumerate()
            .map(|(index, body)| (index, body.name, proximity((self.ship.position - body.center).length(), body.radius)))
            .filter(|(_, _, near)| *near >= ORBIT_ENGAGE_PROXIMITY)
            .max_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(index, name, _)| (index, name))
    }

    /// Entra en modo orbital alrededor del cuerpo más cercano, o vuelve al vuelo
    /// libre. Lejos de todo no hace nada.
    pub fn toggle_orbit(&mut self, scene: &SolarSystem) {
        self.orbit = if self.orbit.is_some() { None } else { self.orbit_candidate(scene) };
    }

    /// Consume `frame_time` segundos reales en pasos fijos.
    pub fn advance(&mut self, scene: &SolarSystem, input: &ShipInput, frame_time: f32) {
        if self.paused {
            return;
        }

        self.accumulator += frame_time.min(MAX_FRAME_TIME);
        while self.accumulator >= STEP {
            self.time += STEP;
            let colliders = scene.colliders(self.time);
            match self.orbit {
                Some((index, _)) => {
                    // En modo orbital no hay gravedad: la altura la decide quien juega.
                    let body = colliders[index];
                    self.ship.step_orbit(input, &body, &colliders, STEP);
                    let distance = (self.ship.position - body.center).length();
                    if proximity(distance, body.radius) < ORBIT_RELEASE_PROXIMITY {
                        self.orbit = None;
                    }
                }
                None => {
                    if self.gravity {
                        self.ship.apply_gravity(&colliders, STEP);
                    }
                    self.ship.step(input, &colliders, STEP);
                }
            }
            let boosting = input.boost && input.thrust > 0.0;
            self.exhaust.step(&self.ship, &scene.ship_model(&self.ship), scene.ship_spec().nozzles, boosting, STEP);
            self.camera.step(&self.ship, &colliders, STEP);
            self.accumulator -= STEP;
        }

        if let Some((index, near)) = scene.nearest_planet(self.time, self.ship.position) {
            if near >= DISCOVERY_PROXIMITY {
                self.discovered[index] = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vuela 3 segundos acelerando y girando, dibujando a `fps` cuadros por segundo.
    fn fly(scene: &SolarSystem, input: &ShipInput, fps: f32) -> Simulation {
        let mut simulation = Simulation::new(scene);
        for _ in 0..(3.0 * fps).round() as usize {
            simulation.advance(scene, input, 1.0 / fps);
        }
        simulation
    }

    /// La misma entrada da el mismo vuelo sin importar cuántos cuadros por
    /// segundo dibuje la computadora. Al terminar, uno de los dos puede tener
    /// un paso pendiente en el acumulador; se completa antes de comparar.
    #[test]
    fn flight_does_not_depend_on_frame_rate() {
        let scene = SolarSystem::load().unwrap();
        let input = ShipInput { thrust: 1.0, yaw: 0.6, pitch: 0.3, roll: -0.4, boost: true };
        let mut slow = fly(&scene, &input, 30.0);
        let mut fast = fly(&scene, &input, 144.0);

        let (behind, ahead) = if slow.time < fast.time { (&mut slow, &fast) } else { (&mut fast, &slow) };
        // Lo que queda en el acumulador es menos de un paso, así que agregar
        // `STEP` avanza exactamente un paso.
        while behind.time < ahead.time - STEP * 0.5 {
            behind.advance(&scene, &input, STEP);
        }

        assert!((slow.time - fast.time).abs() < 1e-4);
        assert!((slow.ship.position - fast.ship.position).length() < 1e-2);
        assert!((slow.ship.forward() - fast.ship.forward()).length() < 1e-3);
    }

    /// Pone la nave a `altitude` sobre el primer planeta.
    fn near_first_planet(scene: &SolarSystem, altitude: f32) -> Simulation {
        let mut simulation = Simulation::new(scene);
        let planet = &PLANETS[0];
        let center = scene.planet_center(planet, 0.0);
        simulation.ship.position = center + crate::ship::Ship::new(center, simulation.ship.orientation).up() * (planet.radius + altitude);
        simulation
    }

    #[test]
    fn orbit_mode_needs_a_body_nearby() {
        let scene = SolarSystem::load().unwrap();
        let mut far = Simulation::new(&scene);
        far.toggle_orbit(&scene);
        assert_eq!(far.orbit, None, "desde el inicio no hay nada cerca");

        let mut near = near_first_planet(&scene, 3.0);
        near.toggle_orbit(&scene);
        assert_eq!(near.orbit.map(|(_, name)| name), Some(PLANETS[0].name));
        near.toggle_orbit(&scene);
        assert_eq!(near.orbit, None, "otra vez sale del modo");
    }

    #[test]
    fn orbit_mode_keeps_the_ship_near_the_moving_planet() {
        let scene = SolarSystem::load().unwrap();
        let mut simulation = near_first_planet(&scene, 3.0);
        simulation.toggle_orbit(&scene);

        let cruise = ShipInput { thrust: 1.0, yaw: 0.3, ..ShipInput::default() };
        for _ in 0..10 * 60 {
            simulation.advance(&scene, &cruise, 1.0 / 60.0);
        }

        let planet = &PLANETS[0];
        let altitude = (simulation.ship.position - scene.planet_center(planet, simulation.time)).length() - planet.radius;
        assert!(simulation.orbit.is_some());
        assert!((altitude - 3.0).abs() < 0.5, "altura {altitude} tras 10 s");
    }

    #[test]
    fn climbing_far_away_leaves_orbit_mode() {
        let scene = SolarSystem::load().unwrap();
        let mut simulation = near_first_planet(&scene, 3.0);
        simulation.toggle_orbit(&scene);
        for _ in 0..15 * 60 {
            simulation.advance(&scene, &ShipInput { pitch: 1.0, ..ShipInput::default() }, 1.0 / 60.0);
        }
        assert_eq!(simulation.orbit, None);
    }

    #[test]
    fn pausing_freezes_the_world() {
        let scene = SolarSystem::load().unwrap();
        let mut simulation = Simulation::new(&scene);
        simulation.paused = true;
        simulation.advance(&scene, &ShipInput { thrust: 1.0, ..ShipInput::default() }, 1.0);
        assert_eq!(simulation.time, 0.0);
        assert_eq!(simulation.ship.speed(), 0.0);
    }
}
