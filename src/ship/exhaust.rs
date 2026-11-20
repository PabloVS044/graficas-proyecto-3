//! Partículas del escape: chispas que salen de los motores con el turbo.
//!
//! Cada partícula nace en una tobera, sale hacia atrás con la velocidad de la
//! nave más un empujón y un poco de desorden, y se va enfriando: pasa de blanco
//! a naranja y a rojo, crece y se apaga. Viven en el mundo, así que quedan como
//! una estela detrás de la nave.
//!
//! Avanzan en los mismos pasos fijos que la física y con un generador de
//! números aleatorios propio con semilla fija: la misma partida da la misma
//! estela en cualquier computadora.

use raylib::math::Vector3;

use super::Ship;
use crate::math::Mat4;
use crate::render::Sprite;

/// Partículas por segundo con el turbo, entre todas las toberas.
const SPAWN_RATE: f32 = 450.0;
/// Velocidad con la que salen hacia atrás, respecto a la nave.
const EJECT_SPEED: f32 = 12.0;
/// Desorden de la velocidad de salida: más alto da un chorro más abierto.
const SPREAD: f32 = 0.9;
/// Cuánto frena cada partícula por segundo (se mezcla con el vacío).
const DRAG: f32 = 2.0;
/// Vida de una partícula, en segundos: entre el mínimo y el máximo.
const LIFETIME_MIN: f32 = 0.3;
const LIFETIME_MAX: f32 = 0.7;
/// Radio al nacer y al morir, en unidades del mundo.
const START_RADIUS: f32 = 0.1;
const END_RADIUS: f32 = 0.32;
/// Tope de partículas vivas, por si el turbo se mantiene mucho tiempo.
const MAX_PARTICLES: usize = 1500;
/// Brillo de una partícula recién nacida.
const BRIGHTNESS: f32 = 1.4;

struct Particle {
    position: Vector3,
    velocity: Vector3,
    age: f32,
    lifetime: f32,
}

pub struct Exhaust {
    particles: Vec<Particle>,
    /// Fracción de partícula que quedó pendiente del paso anterior: con 450 por
    /// segundo y pasos de 1/120 s salen 3.75 por paso.
    pending: f32,
    /// Estado del generador de números aleatorios (xorshift).
    seed: u32,
}

impl Exhaust {
    pub fn new() -> Exhaust {
        Exhaust { particles: Vec::new(), pending: 0.0, seed: 0x9E37_79B9 }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.pending = 0.0;
    }

    /// Número aleatorio entre 0 y 1 (xorshift de 32 bits).
    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed as f32 / u32::MAX as f32
    }

    /// Dirección aleatoria dentro de la esfera de radio 1.
    fn random_direction(&mut self) -> Vector3 {
        Vector3::new(self.random() * 2.0 - 1.0, self.random() * 2.0 - 1.0, self.random() * 2.0 - 1.0)
    }

    /// Avanza las partículas `dt` segundos y, si `boosting`, suelta nuevas desde
    /// las toberas. `model` es la matriz de modelo de la nave y `nozzles`, sus
    /// toberas en coordenadas del OBJ (cada nave tiene las suyas).
    pub fn step(&mut self, ship: &Ship, model: &Mat4, nozzles: &[Vector3], boosting: bool, dt: f32) {
        let drag = (-DRAG * dt).exp();
        for particle in &mut self.particles {
            particle.age += dt;
            particle.velocity *= drag;
            particle.position += particle.velocity * dt;
        }
        self.particles.retain(|particle| particle.age < particle.lifetime);

        if !boosting || nozzles.is_empty() {
            self.pending = 0.0;
            return;
        }

        self.pending += SPAWN_RATE * dt;
        let backwards = -ship.forward();
        while self.pending >= 1.0 && self.particles.len() < MAX_PARTICLES {
            self.pending -= 1.0;
            let nozzle = nozzles[(self.random() * nozzles.len() as f32) as usize % nozzles.len()];
            let velocity =
                ship.velocity + backwards * (EJECT_SPEED * (0.7 + self.random() * 0.6)) + self.random_direction() * SPREAD;
            // Nacen repartidas a lo largo del paso, no todas en el mismo punto.
            let head_start = self.random() * dt;
            let lifetime = LIFETIME_MIN + self.random() * (LIFETIME_MAX - LIFETIME_MIN);
            self.particles.push(Particle {
                position: model.transform_point(nozzle) + velocity * head_start,
                velocity,
                age: head_start,
                lifetime,
            });
        }
    }

    /// Lo que el renderer necesita para dibujarlas.
    pub fn sprites(&self) -> Vec<Sprite> {
        self.particles
            .iter()
            .map(|particle| {
                let t = (particle.age / particle.lifetime).clamp(0.0, 1.0);
                Sprite {
                    position: particle.position,
                    radius: START_RADIUS + (END_RADIUS - START_RADIUS) * t,
                    color: cooling(t) * (BRIGHTNESS * (1.0 - t) * (1.0 - t)),
                }
            })
            .collect()
    }
}

/// Color del escape según su edad (0 recién nacida, 1 apagada): blanco
/// caliente, naranja de los motores y rojo oscuro.
fn cooling(t: f32) -> Vector3 {
    let hot = Vector3::new(1.0, 0.92, 0.75);
    let warm = Vector3::new(1.0, 0.45, 0.15);
    let cool = Vector3::new(0.6, 0.08, 0.04);
    if t < 0.3 {
        hot + (warm - hot) * (t / 0.3)
    } else {
        warm + (cool - warm) * ((t - 0.3) / 0.7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Quat;

    const DT: f32 = 1.0 / 120.0;
    const NOZZLES: &[Vector3] = &[Vector3 { x: 1.0, y: 0.0, z: -2.0 }, Vector3 { x: -1.0, y: 0.0, z: -2.0 }];

    fn ship() -> Ship {
        let mut ship = Ship::new(Vector3::zero(), Quat::IDENTITY);
        ship.velocity = Vector3::new(0.0, 0.0, 20.0);
        ship
    }

    #[test]
    fn only_boosting_makes_particles() {
        let mut exhaust = Exhaust::new();
        for _ in 0..60 {
            exhaust.step(&ship(), &Mat4::IDENTITY, NOZZLES, false, DT);
        }
        assert_eq!(exhaust.len(), 0);
        for _ in 0..60 {
            exhaust.step(&ship(), &Mat4::IDENTITY, NOZZLES, true, DT);
        }
        // Medio segundo a 450 por segundo, menos las que ya murieron.
        assert!(exhaust.len() > 150 && exhaust.len() <= 225, "{}", exhaust.len());
    }

    #[test]
    fn particles_trail_behind_and_die_out() {
        let mut exhaust = Exhaust::new();
        let ship = ship();
        for _ in 0..30 {
            exhaust.step(&ship, &Mat4::IDENTITY, NOZZLES, true, DT);
        }
        // Salen hacia atrás respecto a la nave (la punta mira hacia +z).
        assert!(exhaust.particles.iter().all(|p| (p.velocity - ship.velocity).z < 0.0));

        for _ in 0..(LIFETIME_MAX / DT) as usize + 2 {
            exhaust.step(&ship, &Mat4::IDENTITY, NOZZLES, false, DT);
        }
        assert_eq!(exhaust.len(), 0, "sin turbo se apagan todas");
    }

    #[test]
    fn same_inputs_give_the_same_trail() {
        let run = || {
            let mut exhaust = Exhaust::new();
            for _ in 0..50 {
                exhaust.step(&ship(), &Mat4::IDENTITY, NOZZLES, true, DT);
            }
            exhaust.sprites().iter().map(|s| s.position).collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn particles_cool_down_and_fade() {
        let brightness = |c: Vector3| c.x + c.y + c.z;
        assert!(brightness(cooling(0.0)) > brightness(cooling(0.5)));
        assert!(cooling(1.0).y < 0.1, "al final es rojo");
    }
}
