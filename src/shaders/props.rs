//! Shaders de los objetos de superficie.

use raylib::math::Vector3;

use super::noise_helpers::{fbm, worley};
use super::{color, light_direction, mix, smoothstep};
use crate::render::{Fragment, FrameContext};

/// Poros por unidad del modelo.
const PORE_SCALE: f32 = 9.0;

/// Basalto poroso con lava en las hendiduras y la base al rojo, donde toca el
/// suelo. La profundidad de cada hendidura llega en `uv.x` desde el modelo; los
/// poros finos se calculan aquí con Worley. La lava brilla sola, también de noche.
pub fn basalt(fragment: &Fragment, frame: &FrameContext) -> Vector3 {
    let p = fragment.object_position;
    let grain = fbm(&frame.noise, p * 9.0, 2);
    let rock = mix(color(0.11, 0.09, 0.08), color(0.34, 0.28, 0.24), grain);
    let light = fragment.normal.dot(light_direction(fragment, frame)).max(0.0);

    let (f1, _, _) = worley(p * PORE_SCALE, 77);
    let pore = 1.0 - smoothstep(0.12, 0.3, f1);
    let lava = smoothstep(0.3, 0.7, fragment.uv.x).max(pore * 0.85);
    let base = 1.0 - smoothstep(-0.05, 0.3, p.y);
    let glow = mix(color(0.7, 0.08, 0.0), color(1.0, 0.5, 0.06), lava) * (lava * 1.4)
        + color(0.8, 0.16, 0.0) * (base * 0.9);
    rock * (0.08 + light * 0.92) * (1.0 - lava) + glow
}

/// Cristal de hielo: azul claro, más transparente en los bordes, con reflejo
/// del sol y un brillo cian por dentro que sube desde la base. `glow` va de 0
/// (nave lejos) a 1 (nave encima).
pub fn crystal(fragment: &Fragment, frame: &FrameContext, glow: f32) -> Vector3 {
    let normal = fragment.normal;
    let to_light = light_direction(fragment, frame);
    let view = (frame.camera_position - fragment.world_position).normalize();
    let height = fragment.uv.x;

    let body = mix(color(0.30, 0.55, 0.80), color(0.80, 0.93, 1.0), height);
    let diffuse = normal.dot(to_light).max(0.0);
    let rim = (1.0 - normal.dot(view).abs()).powf(2.0);
    let glint = if diffuse > 0.0 { normal.dot((to_light + view).normalize()).max(0.0).powf(40.0) } else { 0.0 };

    let inner = color(0.15, 1.0, 0.9) * ((0.15 + glow * 1.1) * (0.4 + height * 0.8));
    body * (0.12 + diffuse * 0.6) + color(0.7, 0.9, 1.0) * (rim * 0.5 + glint) + inner
}
