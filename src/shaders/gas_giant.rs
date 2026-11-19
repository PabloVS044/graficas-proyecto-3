//! Gigante gaseoso (Ambar): bandas de nubes que giran a distinta velocidad
//! según la latitud, una gran tormenta en espiral y relámpagos.
//!
//! Reacciona a la nave: al acercarse la tormenta gira más rápido y los
//! relámpagos se vuelven más frecuentes y brillantes.
//!
//! No hay superficie: todo el color sale de deformar las bandas. Las bandas se
//! calculan sobre un punto "movido" de la esfera; mover ese punto (girarlo según
//! la latitud o retorcerlo alrededor de la tormenta) es lo que anima el planeta.

use raylib::math::Vector3;

use super::noise_helpers::{cell_random, fbm};
use super::{color, mix, smoothstep, surface_distance};
use crate::render::{Fragment, FrameContext, ObjectContext};

/// Cuántas bandas claras y oscuras hay de polo a polo.
const BAND_COUNT: f32 = 7.0;
/// Cuánto ondula el borde de las bandas: 0 las deja rectas.
const BAND_TURBULENCE: f32 = 0.45;
/// Radianes por segundo que se adelantan o atrasan las bandas entre sí.
const JET_SPEED: f32 = 0.05;
/// Centro de la tormenta, en el espacio del planeta (se normaliza).
const STORM_CENTER: Vector3 = Vector3 { x: 0.75, y: -0.32, z: 0.58 };
/// Radio de la tormenta, en radianes sobre la superficie.
const STORM_RADIUS: f32 = 0.42;
/// Vueltas por segundo del centro de la tormenta con la nave lejos.
const STORM_SPIN: f32 = 0.04;
/// Cuántas veces más rápido gira con la nave encima.
const STORM_EXCITEMENT: f32 = 4.0;
/// Tamaño de las celdas donde puede caer un relámpago: más alto da más y más pequeñas.
const LIGHTNING_GRID: f32 = 7.0;
/// Destellos por segundo en cada celda con la nave lejos y con la nave encima.
const LIGHTNING_RATE_FAR: f32 = 0.6;
const LIGHTNING_RATE_NEAR: f32 = 3.0;
/// Probabilidad de que una celda destelle en cada intervalo.
const LIGHTNING_CHANCE: f32 = 0.12;

/// Gira `p` un ángulo alrededor de `axis` (largo 1), con la fórmula de Rodrigues.
fn rotate_around(p: Vector3, axis: Vector3, angle: f32) -> Vector3 {
    let (sin, cos) = angle.sin_cos();
    p * cos + axis.cross(p) * sin + axis * (axis.dot(p) * (1.0 - cos))
}

/// Cuánto gira la tormenta en un punto a `distance` radianes de su centro: el
/// centro gira rápido y el borde casi nada, y esa diferencia forma la espiral.
fn storm_twist(distance: f32, time: f32, proximity: f32) -> f32 {
    let inside = 1.0 - smoothstep(0.0, STORM_RADIUS, distance);
    let spin = STORM_SPIN * (1.0 + proximity * STORM_EXCITEMENT);
    inside * inside * (2.5 + time * spin * std::f32::consts::TAU)
}

/// Destello de un relámpago en `p`, de 0 a 1. Cada celda de una cuadrícula
/// sortea en cada intervalo si destella; el destello se apaga rápido.
fn lightning(p: Vector3, time: f32, proximity: f32) -> f32 {
    let cell = p * LIGHTNING_GRID;
    let id = (cell.x.floor() as i32, cell.y.floor() as i32, cell.z.floor() as i32);

    let rate = LIGHTNING_RATE_FAR + (LIGHTNING_RATE_NEAR - LIGHTNING_RATE_FAR) * proximity;
    let clock = time * rate + cell_random(id, 60) * 10.0;
    let interval = clock.floor() as u32;
    if cell_random(id, 61u32.wrapping_add(interval.wrapping_mul(7))) > LIGHTNING_CHANCE {
        return 0.0;
    }

    // Una mancha suave alrededor de un punto de la celda, distinto cada vez.
    let center = Vector3::new(
        id.0 as f32 + 0.2 + cell_random(id, interval.wrapping_add(62)) * 0.6,
        id.1 as f32 + 0.2 + cell_random(id, interval.wrapping_add(63)) * 0.6,
        id.2 as f32 + 0.2 + cell_random(id, interval.wrapping_add(64)) * 0.6,
    );
    let glow = 1.0 - smoothstep(0.0, 0.75, (cell - center).length());
    let fade = (1.0 - clock.fract()).powf(4.0);
    glow * fade
}

pub fn shade(fragment: &Fragment, frame: &FrameContext, object: &ObjectContext) -> Vector3 {
    let noise = &frame.noise;
    let time = frame.time;
    let near = object.proximity;
    let p = fragment.object_position.normalize();

    // Corrientes: cada latitud gira alrededor del eje a su propia velocidad, en
    // sentidos alternados.
    let jet = (p.y * BAND_COUNT * 1.7).sin();
    let flow = rotate_around(p, Vector3::new(0.0, 1.0, 0.0), time * JET_SPEED * jet);

    // Tormenta: retuerce el punto alrededor de su centro.
    let storm_axis = STORM_CENTER.normalize();
    let storm_distance = surface_distance(p, storm_axis);
    let flow = rotate_around(flow, storm_axis, storm_twist(storm_distance, time, near));

    // Bandas: la latitud del punto movido, ondulada con ruido.
    let turbulence = fbm(noise, flow * 3.5, 4) - 0.5;
    let latitude = flow.y + turbulence * BAND_TURBULENCE;
    let band = (latitude * BAND_COUNT * std::f32::consts::PI).sin() * 0.5 + 0.5;
    // Vetas finas estiradas a lo largo de las bandas.
    let streaks = fbm(noise, Vector3::new(flow.x * 4.0, flow.y * 26.0, flow.z * 4.0), 3);

    let rust = color(0.50, 0.26, 0.13);
    let ochre = color(0.82, 0.56, 0.28);
    let cream = color(0.95, 0.87, 0.70);
    let mut clouds = mix(rust, ochre, band);
    clouds = mix(clouds, cream, smoothstep(0.5, 0.75, streaks) * band);

    // La tormenta es más rojiza y su ojo más oscuro.
    let inside = 1.0 - smoothstep(STORM_RADIUS * 0.6, STORM_RADIUS, storm_distance);
    let eye = 1.0 - smoothstep(0.0, STORM_RADIUS * 0.25, storm_distance);
    clouds = mix(clouds, mix(color(0.78, 0.30, 0.15), color(0.95, 0.62, 0.40), streaks), inside * 0.75);
    clouds *= 1.0 - eye * 0.45;

    // Luz del sol, con un borde de atmósfera que se ve también del lado de la
    // cámara.
    let light = fragment.object_normal.dot(object.sun_direction);
    let day = light.max(0.0);
    let view = (frame.camera_position - fragment.world_position).normalize();
    let haze = (1.0 - fragment.normal.dot(view).max(0.0)).powf(2.5);
    let lit = clouds * (0.05 + day * 0.95) + color(1.0, 0.75, 0.45) * (haze * 0.35 * smoothstep(-0.2, 0.4, light));

    // Relámpagos: se notan más del lado de noche y cuando la nave está cerca.
    let night = 1.0 - smoothstep(-0.1, 0.2, light);
    let flash = lightning(p, time, near) * (0.25 + near * 1.2) * (0.55 + night * 0.45);

    lit + color(0.75, 0.85, 1.0) * (flash * 1.6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotating_keeps_points_on_the_sphere() {
        let p = color(0.3, 0.8, -0.52).normalize();
        let axis = STORM_CENTER.normalize();
        let rotated = rotate_around(p, axis, 1.3);
        assert!((rotated.length() - 1.0).abs() < 1e-5);
        assert!((rotated.dot(axis) - p.dot(axis)).abs() < 1e-5, "gira alrededor del eje");
    }

    #[test]
    fn storm_spins_faster_with_the_ship_near() {
        let far = storm_twist(0.0, 10.0, 0.0) - storm_twist(0.0, 0.0, 0.0);
        let near = storm_twist(0.0, 10.0, 1.0) - storm_twist(0.0, 0.0, 1.0);
        assert!(near > far * 3.0);
        assert_eq!(storm_twist(STORM_RADIUS, 10.0, 1.0), 0.0, "el borde no gira");
    }

    #[test]
    fn lightning_is_more_frequent_with_the_ship_near() {
        let flashes = |proximity: f32| {
            (0..3000)
                .filter(|i| {
                    let p = color((*i as f32 * 0.37).sin(), (*i as f32 * 0.11).cos(), 0.4).normalize();
                    lightning(p, *i as f32 * 0.05, proximity) > 0.3
                })
                .count()
        };
        assert!(flashes(1.0) > flashes(0.0));
    }
}
