//! Planeta de hielo (Kryos): océano congelado con campos de nieve, glaciares
//! azules, grietas y auroras en los polos.
//!
//! Reacciona a la nave: bajo el hielo vive algo bioluminiscente. Al acercarse,
//! las grietas se encienden en ondas cian que salen del punto bajo la nave y se
//! extienden más cuanto más cerca está; las auroras también se intensifican.

use raylib::math::Vector3;

use super::noise_helpers::{fbm, sample, worley};
use super::{color, light_direction, mix, smoothstep, surface_distance};
use crate::render::{Fragment, FrameContext, ObjectContext};

/// Tamaño de los bloques de hielo: más alto da más grietas y más juntas.
const CRACK_SCALE: f32 = 6.0;
/// Grosor de las grietas grandes.
const CRACK_WIDTH: f32 = 0.035;
/// Las grietas finas son una segunda red, más densa y más delgada.
const FINE_CRACK_SCALE: f32 = 15.0;
const FINE_CRACK_WIDTH: f32 = 0.05;
/// Ondas por radián de superficie.
const GLOW_WAVE_FREQUENCY: f32 = 14.0;
/// Velocidad de las ondas, en radianes por segundo.
const GLOW_WAVE_SPEED: f32 = 2.2;
/// Hasta dónde llega el brillo con la nave pegada a la superficie, en radianes.
/// A media distancia llega a la mitad.
const GLOW_REACH: f32 = 2.2;
/// Brillo de las grietas con la nave lejos: casi apagadas.
const GLOW_DORMANT: f32 = 0.03;
/// Latitud de las auroras (seno de la latitud: 1 es el polo).
const AURORA_LATITUDE: f32 = 0.8;
/// Ancho de la franja de auroras.
const AURORA_WIDTH: f32 = 0.06;
const SHININESS: f32 = 60.0;

/// Brillo de las grietas en un punto a `distance` radianes del punto bajo la
/// nave. Un área iluminada que crece con la cercanía, y sobre ella ondas que
/// se alejan de la nave.
fn bioluminescence(distance: f32, time: f32, proximity: f32) -> f32 {
    let radius = GLOW_REACH * proximity;
    if radius <= 0.0 {
        return GLOW_DORMANT;
    }
    let area = 1.0 - smoothstep(radius * 0.6, radius, distance);
    let wave = smoothstep(0.55, 1.0, (distance * GLOW_WAVE_FREQUENCY - time * GLOW_WAVE_SPEED).sin() * 0.5 + 0.5);
    GLOW_DORMANT + area * (0.35 + wave * 1.1) * proximity
}

/// Auroras: una franja alrededor de cada polo que ondula con ruido, con rayos
/// verticales que se mueven. Devuelve su color, sin la intensidad general.
fn aurora(noise: &noise::OpenSimplex, p: Vector3, time: f32) -> Vector3 {
    // El ruido se toma en `x`, `z` para que la franja rodee el polo sin costura.
    let pole = p.y.signum();
    let wobble = fbm(noise, Vector3::new(p.x * 3.0 + time * 0.08, pole * 7.0, p.z * 3.0), 2) - 0.5;
    let center = AURORA_LATITUDE + wobble * 0.15;
    let offset = p.y.abs() - center;
    // Más difusa hacia el ecuador, como una cortina que cae.
    let band = 1.0 - smoothstep(0.0, AURORA_WIDTH * (1.0 + (-offset).clamp(0.0, 0.2) * 25.0), offset.abs());
    if band <= 0.0 {
        return Vector3::zero();
    }

    let rays = sample(noise, Vector3::new(p.x * 28.0, time * 0.5, p.z * 28.0)) * 0.5 + 0.5;
    let tint = mix(color(0.2, 1.0, 0.55), color(0.75, 0.3, 1.0), smoothstep(-0.02, 0.06, offset));
    tint * (band * (0.4 + rays * 0.8))
}

pub fn shade(fragment: &Fragment, frame: &FrameContext, object: &ObjectContext) -> Vector3 {
    let noise = &frame.noise;
    let time = frame.time;
    let near = object.proximity;
    let p = fragment.object_position.normalize();

    // Hielo: campos de nieve blanca y glaciares azules.
    let frost = fbm(noise, p * 3.0, 4);
    let ice = mix(color(0.45, 0.66, 0.86), color(0.92, 0.96, 1.0), smoothstep(0.42, 0.62, frost));
    let snow_grain = fbm(noise, p * 30.0, 2);
    let ice = ice * (0.9 + snow_grain * 0.15);

    // Grietas: el borde entre celdas de Worley, en dos tamaños.
    let warp = color(sample(noise, p * 4.0), sample(noise, p * 4.0 + color(9.0, 0.0, 0.0)), 0.0) * 0.06;
    let (near1, near2, _) = worley((p + warp) * CRACK_SCALE, 70);
    let big = 1.0 - smoothstep(CRACK_WIDTH * 0.3, CRACK_WIDTH, near2 - near1);
    let (fine1, fine2, _) = worley(p * FINE_CRACK_SCALE, 73);
    let fine = (1.0 - smoothstep(FINE_CRACK_WIDTH * 0.3, FINE_CRACK_WIDTH, fine2 - fine1)) * 0.5;
    let crack = big.max(fine);
    let surface = mix(ice, color(0.03, 0.16, 0.30), crack * 0.85);

    // Luz: difusa más un reflejo frío del sol.
    let light = fragment.object_normal.dot(object.sun_direction);
    let diffuse = light.max(0.0);
    let to_light = light_direction(fragment, frame);
    let view = (frame.camera_position - fragment.world_position).normalize();
    let glint = if fragment.normal.dot(to_light) > 0.0 {
        fragment.normal.dot((to_light + view).normalize()).max(0.0).powf(SHININESS) * (1.0 - crack)
    } else {
        0.0
    };
    let lit = surface * (0.06 + diffuse * 0.94) + color(0.9, 0.95, 1.0) * (glint * 0.6);

    // Bioluminiscencia en las grietas.
    let distance = surface_distance(p, object.ship_direction);
    let glow = color(0.15, 1.0, 0.9) * (crack * bioluminescence(distance, time, near) * 1.4);

    // Auroras: siempre algo visibles, más de noche y con la nave cerca.
    let night = 1.0 - smoothstep(-0.15, 0.15, light);
    let lights = aurora(noise, p, time) * ((0.15 + near * 0.9) * (0.4 + night * 0.6));

    lit + glow + lights
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cracks_stay_dark_with_the_ship_far() {
        for i in 0..50 {
            assert_eq!(bioluminescence(i as f32 * 0.06, i as f32, 0.0), GLOW_DORMANT);
        }
    }

    #[test]
    fn glow_spreads_further_the_closer_the_ship() {
        // Cerca de la nave, el área iluminada cubre el punto; lejos de ella, no.
        let at = |distance: f32, proximity: f32| {
            (0..100).map(|i| bioluminescence(distance, i as f32 * 0.1, proximity)).fold(0.0, f32::max)
        };
        assert!(at(1.5, 1.0) > 0.5);
        assert_eq!(at(1.5, 0.4), GLOW_DORMANT);
    }

    #[test]
    fn auroras_only_near_the_poles() {
        let noise = noise::OpenSimplex::new(7);
        let equator = color(1.0, 0.0, 0.0);
        assert_eq!(aurora(&noise, equator, 0.0), Vector3::zero());
        let strongest = (0..200)
            .map(|i| {
                let angle = i as f32 * 0.0314;
                let y = AURORA_LATITUDE;
                let r = (1.0 - y * y).sqrt();
                let c = aurora(&noise, color(r * angle.cos(), y, r * angle.sin()), 0.0);
                c.x + c.y + c.z
            })
            .fold(0.0, f32::max);
        assert!(strongest > 0.3);
    }
}
