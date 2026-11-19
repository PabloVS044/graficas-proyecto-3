//! Shaders de los objetos de superficie.

use raylib::math::Vector3;

use super::noise_helpers::{fbm, sample, worley};
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

/// Cuánto le llega el sol a un fragmento, con un mínimo para el lado de noche.
fn sunlight(fragment: &Fragment, frame: &FrameContext) -> f32 {
    0.07 + fragment.normal.dot(light_direction(fragment, frame)).max(0.0) * 0.93
}

/// Qué tan de noche es donde está el objeto: 1 si el suelo no ve el sol.
fn night(ground_up: Vector3, fragment: &Fragment, frame: &FrameContext) -> f32 {
    1.0 - smoothstep(-0.1, 0.15, ground_up.dot(light_direction(fragment, frame)))
}

/// Paja del techo: hebras que bajan desde la punta, en tonos de heno. Los
/// palos de la punta (`uv.y` = 1) son madera oscura.
pub fn thatch(fragment: &Fragment, frame: &FrameContext) -> Vector3 {
    if fragment.uv.y > 0.5 {
        return color(0.22, 0.13, 0.06) * sunlight(fragment, frame);
    }
    let p = fragment.object_position;
    let around = p.z.atan2(p.x);
    let strands = (around * 60.0 + sample(&frame.noise, p * 12.0) * 3.0).sin() * 0.5 + 0.5;
    // Capas de paja: franjas horizontales donde una capa se monta en la otra.
    let layer = smoothstep(0.75, 1.0, (fragment.uv.x * 5.0).fract());
    let straw = mix(color(0.42, 0.32, 0.14), color(0.66, 0.53, 0.26), strands * 0.7 + fbm(&frame.noise, p * 20.0, 2) * 0.3);
    straw * (1.0 - layer * 0.45) * sunlight(fragment, frame)
}

/// Pared de barro con palos y una puerta por donde se ve el fuego de adentro.
/// La puerta queda en `uv.x` = 0 (el ángulo alrededor de la choza).
pub fn hut_wall(fragment: &Fragment, frame: &FrameContext, ground_up: Vector3) -> Vector3 {
    let p = fragment.object_position;
    let mud = mix(color(0.30, 0.20, 0.11), color(0.46, 0.33, 0.19), fbm(&frame.noise, p * 16.0, 3));
    // Palos verticales metidos en el barro.
    let around = fragment.uv.x;
    let sticks = smoothstep(0.7, 0.9, (around * 40.0 + sample(&frame.noise, p * 6.0) * 0.6).sin());
    let wall = mix(mud, color(0.24, 0.15, 0.08), sticks * 0.6) * sunlight(fragment, frame);

    let from_door = around.min(1.0 - around);
    let door = (1.0 - smoothstep(0.035, 0.05, from_door)) * (1.0 - smoothstep(0.24, 0.28, p.y));
    let fire = color(1.0, 0.45, 0.1) * (0.35 + night(ground_up, fragment, frame) * 0.9);
    mix(wall, fire, door)
}
