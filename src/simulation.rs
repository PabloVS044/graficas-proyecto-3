//! Avance del mundo en pasos fijos.
//!
//! Cada cuadro dura lo que tarde la computadora en dibujarlo. Si la física
//! avanzara con ese tiempo variable, la nave respondería distinto en cada
//! equipo. En cambio, el tiempo real se acumula y se consume en pasos de
//! duración fija: en una computadora lenta se dan varios pasos por cuadro, y
//! en una rápida a veces ninguno, pero el resultado es el mismo.

use crate::scene::SolarSystem;
use crate::scene::planets::PLANETS;
use crate::ship::{ChaseCamera, Ship, ShipInput};

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
    accumulator: f32,
}

/// Qué tan cerca hay que llegar a un planeta para contarlo como descubierto.
const DISCOVERY_PROXIMITY: f32 = 0.6;

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
            accumulator: 0.0,
        }
    }

    /// Vuelve la nave al inicio sin detener las órbitas.
    pub fn reset_ship(&mut self, scene: &SolarSystem) {
        self.ship = scene.ship_start();
        self.camera = ChaseCamera::behind(&self.ship);
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
            if self.gravity {
                self.ship.apply_gravity(&colliders, STEP);
            }
            self.ship.step(input, &colliders, STEP);
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
