//! Planeta volcánico (Ignis): corteza de basalto partida en placas, lava que
//! corre por las grietas y volcanes que erupcionan.
//!
//! Reacciona a la nave: al acercarse las grietas se abren, la lava se calienta,
//! los volcanes erupcionan más seguido y salen ondas de calor desde el punto de
//! la superficie que queda debajo de la nave. El casco de la nave también se
//! calienta (ver `SolarSystem::ship_heat` y el shader de la nave).
//!
//! Todo se calcula sobre la esfera de radio 1 en el espacio del planeta, así
//! que las placas giran con él.

use std::f32::consts::TAU;

use raylib::math::Vector3;

use super::noise_helpers::{cell_random, fbm, sample, worley};
use super::{color, mix, smoothstep, surface_distance};
use crate::render::{Fragment, FrameContext, ObjectContext};

/// Tamaño de las placas: más alto da más placas y más pequeñas.
const PLATE_SCALE: f32 = 5.0;
/// Grosor de las grietas con la nave lejos.
const CRACK_WIDTH: f32 = 0.06;
/// Cuánto se abren las grietas con la nave pegada a la superficie.
const CRACK_OPENING: f32 = 0.05;
/// Velocidad a la que corre la lava por las grietas.
const FLOW_SPEED: f32 = 0.25;
/// Fracción de placas con un volcán en el centro.
const VOLCANO_CHANCE: f32 = 0.2;
/// Erupciones por segundo de cada volcán con la nave lejos (en promedio).
const ERUPTION_RATE: f32 = 0.35;
/// Cuántas veces más rápido erupcionan con la nave encima.
const ERUPTION_EXCITEMENT: f32 = 4.0;
/// Ondas de calor por radián de superficie.
const HEAT_WAVE_FREQUENCY: f32 = 10.0;
/// Velocidad a la que las ondas se alejan de la nave, en radianes por segundo.
const HEAT_WAVE_SPEED: f32 = 3.0;
/// Hasta dónde llegan las ondas desde el punto bajo la nave, en radianes.
const HEAT_WAVE_REACH: f32 = 1.4;
/// Brillo general de la lava: 0 la apaga.
const LAVA_BRIGHTNESS: f32 = 1.6;

/// Color de la lava según su temperatura: de rojo oscuro (0) a naranja (0.5) y
/// amarillo casi blanco (1).
fn lava_color(temperature: f32) -> Vector3 {
    let t = temperature.clamp(0.0, 1.0);
    let cool = color(0.45, 0.02, 0.0);
    let warm = color(1.0, 0.32, 0.02);
    let hot = color(1.0, 0.86, 0.45);
    if t < 0.5 { mix(cool, warm, t * 2.0) } else { mix(warm, hot, (t - 0.5) * 2.0) }
}

/// Ondas que salen del punto bajo la nave. `distance` es la distancia a ese
/// punto en radianes. Devuelve de 0 a 1.
fn heat_wave(distance: f32, time: f32, proximity: f32) -> f32 {
    let ring = ((distance * HEAT_WAVE_FREQUENCY - time * HEAT_WAVE_SPEED).sin() * 0.5 + 0.5).powf(3.0);
    let reach = 1.0 - smoothstep(0.0, HEAT_WAVE_REACH, distance);
    ring * reach * proximity
}

pub fn shade(fragment: &Fragment, frame: &FrameContext, object: &ObjectContext) -> Vector3 {
    let noise = &frame.noise;
    let time = frame.time;
    let near = object.proximity;
    let p = fragment.object_position.normalize();

    // Placas: el borde entre dos celdas de Worley es la grieta. Un poco de ruido
    // dobla los bordes para que no se vean rectos.
    let warp = Vector3::new(
        sample(noise, p * 3.0),
        sample(noise, p * 3.0 + color(17.0, 0.0, 0.0)),
        sample(noise, p * 3.0 + color(0.0, 29.0, 0.0)),
    ) * 0.08;
    let (nearest, second, plate) = worley((p + warp) * PLATE_SCALE, 40);
    let edge = second - nearest;
    let width = CRACK_WIDTH + CRACK_OPENING * near;
    let crack = 1.0 - smoothstep(width * 0.4, width, edge);
    // La corteza junto a una grieta está tibia y brilla apenas.
    let scorched = 1.0 - smoothstep(width, width * 3.5, edge);

    // Corteza: cada placa tiene su tono, más un grano fino de ceniza.
    let tone = cell_random(plate, 41);
    let grain = fbm(noise, p * 18.0, 3);
    let crust = mix(color(0.05, 0.045, 0.045), color(0.18, 0.14, 0.12), tone * 0.6 + grain * 0.4);

    // Temperatura de la lava: un ruido que se desplaza con el tiempo hace que
    // las zonas calientes avancen por las grietas.
    let flow = fbm(noise, p * 7.0 + color(0.0, time * FLOW_SPEED, time * FLOW_SPEED * 0.6), 3);
    let distance = surface_distance(p, object.ship_direction);
    let temperature = 0.3 + flow * 0.55 + heat_wave(distance, time, near) * 0.7 + near * 0.1;

    // Volcanes: algunas placas tienen un cráter en su centro que erupciona de
    // vez en cuando. La nave cerca los altera.
    let volcano = if cell_random(plate, 42) < VOLCANO_CHANCE {
        let phase = cell_random(plate, 43) * TAU;
        let rate = ERUPTION_RATE * (1.0 + near * ERUPTION_EXCITEMENT);
        let burst = ((time * rate * TAU + phase).sin() * 0.5 + 0.5).powf(6.0);
        let crater = 1.0 - smoothstep(0.06, 0.3, nearest);
        crater * (0.3 + burst * (1.2 + near))
    } else {
        0.0
    };

    let heat = crack * temperature + scorched * temperature * 0.12 + volcano;
    let lava = lava_color(heat * 0.8) * (heat * LAVA_BRIGHTNESS);

    // La corteza recibe la luz del sol; la lava brilla sola, también de noche.
    let light = fragment.object_normal.dot(object.sun_direction).max(0.0);
    crust * (0.07 + light * 0.93) * (1.0 - crack) + lava
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lava_gets_brighter_as_it_heats_up() {
        let brightness = |t: f32| {
            let c = lava_color(t);
            c.x + c.y + c.z
        };
        assert!(brightness(0.0) < brightness(0.5) && brightness(0.5) < brightness(1.0));
    }

    #[test]
    fn heat_waves_only_appear_with_the_ship_near() {
        let strongest = |proximity: f32| {
            (0..400).map(|i| heat_wave(i as f32 * 0.003, 1.0, proximity)).fold(0.0, f32::max)
        };
        assert_eq!(strongest(0.0), 0.0);
        assert!(strongest(1.0) > 0.5);
        // Lejos del punto bajo la nave no llegan.
        assert_eq!(heat_wave(HEAT_WAVE_REACH + 0.1, 1.0, 1.0), 0.0);
    }
}
