//! El sol: plasma que hierve, granulación, manchas solares y filamentos
//! brillantes. No recibe luz: emite su propio color.
//!
//! Reacciona a la nave: al acercarse el plasma se agita más rápido, los
//! filamentos se encienden y todo brilla más. El casco de la nave también se
//! calienta (ver `SolarSystem::ship_heat`).

use raylib::math::Vector3;

use super::noise_helpers::{fbm, sample, worley};
use super::{color, mix, smoothstep};
use crate::render::{Fragment, FrameContext, ObjectContext};

/// Tamaño de los gránulos de convección: más alto da más y más pequeños.
const GRANULE_SCALE: f32 = 16.0;
/// Velocidad con la que hierve el plasma con la nave lejos.
const BOIL_SPEED: f32 = 0.12;
/// Cuántas veces más rápido hierve con la nave encima.
const BOIL_EXCITEMENT: f32 = 2.5;
/// Valor del ruido desde el que hay mancha solar: más alto da menos manchas.
const SUNSPOT_THRESHOLD: f32 = 0.64;
/// Grosor de los filamentos.
const FILAMENT_WIDTH: f32 = 0.035;
/// Brillo extra con la nave pegada a la superficie.
const PROXIMITY_GLARE: f32 = 0.5;

/// Color del plasma según su temperatura: naranja rojizo (0), amarillo (0.5) y
/// blanco cálido (1).
fn plasma_color(temperature: f32) -> Vector3 {
    let t = temperature.clamp(0.0, 1.0);
    let cool = color(0.85, 0.25, 0.02);
    let warm = color(1.0, 0.72, 0.22);
    let hot = color(1.0, 0.97, 0.85);
    if t < 0.5 { mix(cool, warm, t * 2.0) } else { mix(warm, hot, (t - 0.5) * 2.0) }
}

pub fn shade(fragment: &Fragment, frame: &FrameContext, object: &ObjectContext) -> Vector3 {
    let noise = &frame.noise;
    let near = object.proximity;
    let p = fragment.object_position.normalize();
    // El tiempo del plasma corre más rápido con la nave cerca.
    let boil = frame.time * BOIL_SPEED * (1.0 + near * BOIL_EXCITEMENT);

    // Turbulencia grande que deforma todo lo demás.
    let warp = fbm(noise, p * 2.5 + color(0.0, boil, 0.0), 3) - 0.5;
    let q = p + Vector3::new(warp, -warp, warp) * 0.3;

    // Granulación: el centro de cada celda de convección es más caliente que
    // su borde. Las celdas se desplazan con el tiempo.
    let (nearest, second, _) = worley(q * GRANULE_SCALE + color(boil * 2.0, 0.0, -boil), 90);
    let granule = smoothstep(0.0, 0.5, second - nearest);

    let plasma = fbm(noise, q * 5.0 + color(-boil, boil * 0.5, 0.0), 4);
    let mut temperature = 0.4 + (plasma - 0.5) * 0.9 + granule * 0.18;

    // Manchas solares: zonas más frías y oscuras que se mueven muy despacio.
    let spots = smoothstep(SUNSPOT_THRESHOLD, SUNSPOT_THRESHOLD + 0.06, fbm(noise, p * 2.2 + color(5.0, boil * 0.2, 9.0), 3));
    temperature -= spots * 0.55;

    // Filamentos: líneas donde el ruido cruza por 0, como los ríos de Jungla.
    // Casi no se ven de lejos y se encienden con la nave cerca.
    let filament_line = sample(noise, q * 6.0 + color(boil, 0.0, boil)).abs();
    let filament = 1.0 - smoothstep(FILAMENT_WIDTH * 0.4, FILAMENT_WIDTH, filament_line);
    temperature += filament * (0.1 + near * 0.35);

    // Oscurecimiento del borde: el centro del disco se ve más caliente que la
    // orilla, como en el sol de verdad.
    let view = (frame.camera_position - fragment.world_position).normalize();
    let facing = fragment.normal.dot(view).max(0.0);
    let limb = 0.65 + 0.35 * facing.sqrt();

    plasma_color(temperature) * (limb * (1.15 + near * PROXIMITY_GLARE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotter_plasma_is_brighter() {
        let brightness = |t: f32| {
            let c = plasma_color(t);
            c.x + c.y + c.z
        };
        assert!(brightness(0.0) < brightness(0.5) && brightness(0.5) < brightness(1.0));
    }
}
