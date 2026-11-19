//! Planeta dormido (Eco): roca morada cubierta por una red hexagonal tallada,
//! los restos de algo construido hace mucho.
//!
//! Reacciona a la nave: al acercarse la red se despierta. Los hexágonos se
//! encienden uno por uno desde el punto bajo la nave, con pulsos que lo
//! recorren como un escaneo, y en algunos se iluminan glifos. Al alejarse se
//! apaga en el orden contrario.
//!
//! La red hexagonal es plana, así que se proyecta sobre la esfera por las seis
//! caras de un cubo: cada punto usa la cara hacia la que más apunta.

use raylib::math::Vector3;

use super::noise_helpers::{cell_random, fbm};
use super::{color, mix, smoothstep, surface_distance};
use crate::render::{Fragment, FrameContext, ObjectContext};

/// Hexágonos a lo ancho de cada cara del cubo.
const HEX_SCALE: f32 = 7.0;
/// Grosor de las líneas de la red, en unidades de un hexágono.
const LINE_WIDTH: f32 = 0.05;
/// Qué tan lejos llega lo despierto con la nave pegada a la superficie, en
/// radianes. Más de π cubre todo el planeta.
const WAKE_REACH: f32 = 3.4;
/// Cuánto se adelanta o atrasa cada hexágono al encenderse, en radianes. Sin
/// esto el borde de lo despierto sería un círculo perfecto.
const WAKE_JITTER: f32 = 0.45;
/// Fracción de hexágonos con un glifo.
const GLYPH_CHANCE: f32 = 0.3;
/// Pulsos de escaneo por radián y su velocidad en radianes por segundo.
const SCAN_FREQUENCY: f32 = 6.0;
const SCAN_SPEED: f32 = 2.5;

/// Coordenadas en la cara del cubo hacia la que apunta `p`, y el número de cara.
fn cube_face(p: Vector3) -> (f32, f32, i32) {
    let a = Vector3::new(p.x.abs(), p.y.abs(), p.z.abs());
    if a.x >= a.y && a.x >= a.z {
        (p.z / a.x, p.y / a.x, if p.x > 0.0 { 0 } else { 1 })
    } else if a.y >= a.z {
        (p.x / a.y, p.z / a.y, if p.y > 0.0 { 2 } else { 3 })
    } else {
        (p.x / a.z, p.y / a.z, if p.z > 0.0 { 4 } else { 5 })
    }
}

/// Hexágono que contiene a `(u, v)`. Devuelve la distancia al borde (0 en el
/// borde, 0.5 en el centro), la distancia al centro y un identificador.
///
/// Los centros de una red hexagonal forman dos cuadrículas rectangulares
/// desplazadas media celda; el centro más cercano de las dos es el del hexágono.
fn hexagon(u: f32, v: f32) -> (f32, f32, (i32, i32)) {
    const H: f32 = 1.732_050_8;
    let offset = |x: f32, y: f32| (x.rem_euclid(1.0) - 0.5, y.rem_euclid(H) - H * 0.5);
    let a = offset(u, v);
    let b = offset(u - 0.5, v - H * 0.5);
    let length = |(x, y): (f32, f32)| (x * x + y * y).sqrt();
    let local = if length(a) < length(b) { a } else { b };

    let (x, y) = (local.0.abs(), local.1.abs());
    let hex_distance = (x * 0.5 + y * 0.866_025_4).max(x);
    let id = (((u - local.0) * 2.0).round() as i32, ((v - local.1) * 2.0).round() as i32);
    (0.5 - hex_distance, length(local), id)
}

/// Qué tan despierto está un hexágono a `distance` radianes de la nave, de 0 a 1.
/// `jitter` (de 0 a 1) lo adelanta o atrasa para que no se enciendan todos a la vez.
fn wakefulness(distance: f32, jitter: f32, proximity: f32) -> f32 {
    let reach = WAKE_REACH * proximity;
    1.0 - smoothstep(reach - 0.3, reach, distance + jitter * WAKE_JITTER)
}

pub fn shade(fragment: &Fragment, frame: &FrameContext, object: &ObjectContext) -> Vector3 {
    let noise = &frame.noise;
    let time = frame.time;
    let near = object.proximity;
    let p = fragment.object_position.normalize();

    let (u, v, face) = cube_face(p);
    let (edge, from_center, cell) = hexagon(u * HEX_SCALE, v * HEX_SCALE);
    let id = (cell.0, cell.1, face);

    // Roca: morada, con manchas grandes y polvo fino. Las líneas de la red
    // están talladas, un poco más oscuras.
    let rock = mix(
        color(0.13, 0.10, 0.18),
        color(0.34, 0.27, 0.42),
        fbm(noise, p * 4.0, 4) * 0.7 + fbm(noise, p * 25.0, 2) * 0.3,
    );
    let line = 1.0 - smoothstep(LINE_WIDTH * 0.5, LINE_WIDTH, edge);
    let rock = rock * (1.0 - line * 0.35);

    let light = fragment.object_normal.dot(object.sun_direction).max(0.0);
    let lit = rock * (0.07 + light * 0.93);

    // Despertar: cada hexágono se enciende según su distancia a la nave.
    let distance = surface_distance(p, object.ship_direction);
    let awake = wakefulness(distance, cell_random(id, 80), near);
    if awake <= 0.0 {
        return lit;
    }

    // Pulsos de escaneo que se alejan de la nave.
    let scan = smoothstep(0.8, 1.0, (distance * SCAN_FREQUENCY - time * SCAN_SPEED).sin() * 0.5 + 0.5);
    // Cada hexágono respira a su propio ritmo.
    let breath = 0.7 + 0.3 * (time * 1.5 + cell_random(id, 81) * 6.28).sin();

    let lines = color(0.55, 0.35, 1.0) * (line * (0.7 + scan * 1.2) * breath);
    // El interior del hexágono se tiñe apenas, más con el pulso.
    let fill = color(0.25, 0.12, 0.45) * ((0.12 + scan * 0.35) * (1.0 - line));

    // Glifos: un anillo y un punto en el centro de algunos hexágonos.
    let glyph = if cell_random(id, 82) < GLYPH_CHANCE {
        let ring_radius = 0.18 + cell_random(id, 83) * 0.12;
        let ring = 1.0 - smoothstep(0.015, 0.04, (from_center - ring_radius).abs());
        let dot = 1.0 - smoothstep(0.03, 0.06, from_center);
        color(0.3, 1.0, 0.85) * ((ring + dot) * breath * 1.2)
    } else {
        Vector3::zero()
    };

    lit + (lines + fill + glyph) * awake
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hexagon_centers_are_far_from_the_edge() {
        let (edge, from_center, _) = hexagon(0.0, 0.0);
        assert!((edge - 0.5).abs() < 1e-4 && from_center < 1e-4);
        // A medio camino entre dos centros vecinos (0,0) y (1,0) está el borde.
        let (edge, _, _) = hexagon(0.5, 0.0);
        assert!(edge.abs() < 1e-4);
    }

    #[test]
    fn neighbouring_hexagons_have_different_ids() {
        let (_, _, a) = hexagon(0.1, 0.0);
        let (_, _, b) = hexagon(0.9, 0.0);
        let (_, _, c) = hexagon(0.15, 0.05);
        assert_ne!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn every_point_lands_on_one_cube_face() {
        for (p, face) in [(color(1.0, 0.2, 0.1), 0), (color(-0.1, -1.0, 0.3), 3), (color(0.2, 0.1, 1.0), 4)] {
            let (u, v, f) = cube_face(p.normalize());
            assert_eq!(f, face);
            assert!(u.abs() <= 1.0 && v.abs() <= 1.0);
        }
    }

    #[test]
    fn wakes_up_near_the_ship_and_sleeps_when_it_leaves() {
        assert_eq!(wakefulness(0.0, 0.0, 0.0), 0.0, "dormido con la nave lejos");
        assert_eq!(wakefulness(0.1, 1.0, 1.0), 1.0, "despierto bajo la nave");
        assert_eq!(wakefulness(2.0, 0.0, 0.3), 0.0, "a media distancia aún no llega tan lejos");
    }
}
