//! Luna de roca gris: mares oscuros de lava vieja, cráteres de varios tamaños
//! con el borde levantado y polvo fino.
//!
//! Los cráteres salen de ruido de Worley en dos escalas: cada celda tiene un
//! punto, y la distancia a él decide si estamos en el fondo del cráter, en su
//! borde o fuera. El tamaño de cada cráter es un número aleatorio de su celda.

use raylib::math::Vector3;

use super::noise_helpers::{cell_random, fbm, worley};
use super::{color, mix, smoothstep};
use crate::render::{Fragment, FrameContext, ObjectContext};

/// Tamaño de los cráteres grandes y de los chicos: más alto da más y más pequeños.
const LARGE_CRATERS: f32 = 4.0;
const SMALL_CRATERS: f32 = 11.0;
/// Fracción de celdas que tienen cráter.
const CRATER_CHANCE: f32 = 0.6;
/// Cuánto oscurecen los mares.
const MARE_DARKNESS: f32 = 0.45;
/// Luz mínima del lado de noche.
const AMBIENT: f32 = 0.04;

/// Cuánto cambia el brillo en un punto por un cráter de una escala: negativo en
/// el fondo (sombra), positivo en el borde levantado. Cero fuera del cráter.
fn crater(p: Vector3, scale: f32, salt: u32) -> f32 {
    let (distance, _, cell) = worley(p * scale, salt);
    if cell_random(cell, salt + 3) > CRATER_CHANCE {
        return 0.0;
    }
    let radius = 0.18 + cell_random(cell, salt + 4) * 0.25;
    let floor = 1.0 - smoothstep(radius * 0.6, radius * 0.9, distance);
    let rim = smoothstep(radius * 0.75, radius, distance) * (1.0 - smoothstep(radius, radius * 1.35, distance));
    rim * 0.35 - floor * 0.3
}

pub fn shade(fragment: &Fragment, frame: &FrameContext, object: &ObjectContext) -> Vector3 {
    let noise = &frame.noise;
    let p = fragment.object_position.normalize();

    // Roca: un gris con manchas grandes, y mares más oscuros.
    let rock = mix(color(0.42, 0.41, 0.40), color(0.68, 0.67, 0.65), fbm(noise, p * 3.0, 4));
    let mare = smoothstep(0.52, 0.6, fbm(noise, p * 1.6 + color(40.0, 3.0, 9.0), 3));
    let dust = fbm(noise, p * 40.0, 2);
    let albedo = rock * (1.0 - mare * MARE_DARKNESS) * (0.85 + dust * 0.25);

    let relief = crater(p, LARGE_CRATERS, 100) + crater(p, SMALL_CRATERS, 110) * 0.7;
    let albedo = albedo * (1.0 + relief).max(0.2);

    let light = fragment.object_normal.dot(object.sun_direction).max(0.0);
    albedo * (AMBIENT + light * (1.0 - AMBIENT))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// En algún punto hay fondo de cráter (más oscuro) y en otro borde (más claro).
    #[test]
    fn craters_have_dark_floors_and_bright_rims() {
        let values: Vec<f32> = (0..4000)
            .map(|i| {
                let angle = i as f32 * 0.0016;
                crater(color(angle.cos(), 0.3, angle.sin()).normalize(), LARGE_CRATERS, 100)
            })
            .collect();
        assert!(values.iter().any(|v| *v < -0.2));
        assert!(values.iter().any(|v| *v > 0.2));
    }
}
