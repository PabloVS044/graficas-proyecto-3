//! Shader de la nave: casco blanco desgastado con franjas rojas, cañones de
//! metal y motores encendidos.
//!
//! Todo se decide con la posición en el espacio del modelo, así que la pintura
//! viaja pegada a la nave. Zonas del modelo (unidades del OBJ):
//!
//! - fuselaje: `|x| < 1`, de la cola (`z = -2.2`) a la punta (`z = 5.4`);
//! - alas: `1 < |x| < 9`;
//! - cañones en las puntas de las alas: `|x| > 9`.

use raylib::math::Vector3;

use super::noise_helpers::{fbm, sample};
use super::{color, light_direction, mix, smoothstep};
use crate::render::{Fragment, FrameContext};

/// Pintura base del casco.
const HULL: Vector3 = Vector3 { x: 0.86, y: 0.84, z: 0.79 };
const STRIPE: Vector3 = Vector3 { x: 0.72, y: 0.16, z: 0.11 };
const METAL: Vector3 = Vector3 { x: 0.42, y: 0.43, z: 0.46 };
const GRIME: Vector3 = Vector3 { x: 0.36, y: 0.31, z: 0.25 };
const ENGINE: Vector3 = Vector3 { x: 1.0, y: 0.42, z: 0.18 };
const RIM: Vector3 = Vector3 { x: 0.45, y: 0.6, z: 0.9 };

/// Cuánto ensucia el desgaste: 0 deja la nave nueva.
const WEAR: f32 = 0.55;
/// Fracción de la pintura roja que se ha caído.
const CHIPPING: f32 = 0.3;
/// Luz que la nave recibe aunque no la ilumine el sol.
const AMBIENT: f32 = 0.14;
/// Fuerza de la luz que dibuja la silueta.
const RIM_STRENGTH: f32 = 0.45;
const SHININESS: f32 = 40.0;
/// Brillo de los motores con el acelerador a fondo.
const ENGINE_BRIGHTNESS: f32 = 1.5;
/// Desde qué calor empieza a brillar el casco: por debajo no se nota.
const HEAT_START: f32 = 0.25;
/// Brillo del casco al rojo vivo.
const HEAT_BRIGHTNESS: f32 = 1.4;

/// `throttle` va de 0 (motores en reposo) a 1 (turbo); `heat`, de 0 (casco
/// frío) a 1 (al rojo vivo).
pub fn shade(fragment: &Fragment, frame: &FrameContext, throttle: f32, heat: f32) -> Vector3 {
    let albedo = paint(fragment.object_position, frame);

    // Luz: difusa del sol, un poco de ambiente, reflejo y borde.
    let normal = fragment.normal;
    let to_light = light_direction(fragment, frame);
    let to_camera = (frame.camera_position - fragment.world_position).normalize();
    let diffuse = normal.dot(to_light).max(0.0);
    let half = (to_light + to_camera).normalize();
    let specular = normal.dot(half).max(0.0).powf(SHININESS) * (diffuse > 0.0) as u8 as f32;
    let rim = (1.0 - normal.dot(to_camera).max(0.0)).powf(3.0);

    let lit = albedo * (AMBIENT + diffuse * (1.0 - AMBIENT)) + Vector3::one() * (specular * 0.35) + RIM * (rim * RIM_STRENGTH);

    lit + engine_glow(fragment, frame, throttle) + hull_heat(fragment, frame, heat, rim)
}

/// Brillo del casco caliente. Empieza por la silueta, donde el metal es más
/// delgado visto desde la cámara, y con más calor cubre toda la nave. El ruido
/// que sube con el tiempo imita el aire caliente que tiembla.
fn hull_heat(fragment: &Fragment, frame: &FrameContext, heat: f32, rim: f32) -> Vector3 {
    let amount = smoothstep(HEAT_START, 1.0, heat);
    if amount <= 0.0 {
        return Vector3::zero();
    }

    let p = fragment.object_position;
    let shimmer = 0.75 + 0.25 * sample(&frame.noise, Vector3::new(p.x * 1.3, p.y * 1.3 - frame.time * 4.0, p.z * 1.3));
    let cover = rim * 0.9 + amount * 0.35;
    mix(color(0.55, 0.04, 0.0), color(1.0, 0.45, 0.1), amount) * (amount * cover * shimmer * HEAT_BRIGHTNESS)
}

/// Color de la superficie, antes de iluminar.
fn paint(p: Vector3, frame: &FrameContext) -> Vector3 {
    let noise = &frame.noise;
    let side = p.x.abs();

    let mut albedo = if side > 9.0 {
        // Cañones: metal, más oscuro hacia la boca.
        METAL * (1.0 - smoothstep(2.5, 5.0, p.z) * 0.45)
    } else {
        HULL
    };

    // Franjas rojas: a los costados del fuselaje y dos barras en cada ala.
    let fuselage_stripe = (side < 1.0) as u8 as f32
        * band(p.z, 0.4, 4.6, 0.15)
        * band(p.y, -0.15, 0.22, 0.04);
    let wing_stripes = (side > 1.0 && side < 9.0) as u8 as f32
        * (band(side, 6.3, 7.0, 0.05) + band(side, 7.5, 8.0, 0.05)).min(1.0);
    let chipped = smoothstep(CHIPPING - 0.05, CHIPPING + 0.05, fbm(noise, p * 3.1 + Vector3::new(7.0, 1.0, 3.0), 3));
    albedo = mix(albedo, STRIPE, (fuselage_stripe + wing_stripes).min(1.0) * chipped);

    // Líneas de paneles: una cuadrícula fina sobre el casco.
    let cell = |v: f32| {
        let f = (v * 1.6).fract().abs();
        f.min(1.0 - f)
    };
    let seam = 1.0 - smoothstep(0.0, 0.03, cell(p.x).min(cell(p.z)));
    albedo *= 1.0 - seam * 0.25 * (side < 9.0) as u8 as f32;

    // Suciedad: manchas grandes y polvo fino.
    let stains = smoothstep(0.45, 0.75, fbm(noise, p * 0.9, 4));
    let dust = fbm(noise, p * 9.0, 2);
    mix(albedo, GRIME, (stains * 0.7 + dust * 0.3) * WEAR)
}

/// 1 entre `start` y `end`, con bordes suaves de ancho `soft`.
fn band(value: f32, start: f32, end: f32, soft: f32) -> f32 {
    smoothstep(start - soft, start + soft, value) * (1.0 - smoothstep(end - soft, end + soft, value))
}

/// Toberas: la parte trasera de las cápsulas de las puntas de las alas y la
/// cola del fuselaje, en las caras que miran hacia atrás. Brillan más con el
/// acelerador y parpadean un poco con el tiempo.
fn engine_glow(fragment: &Fragment, frame: &FrameContext, throttle: f32) -> Vector3 {
    let p = fragment.object_position;
    let side = p.x.abs();
    let pods = smoothstep(9.2, 9.6, side) * (1.0 - smoothstep(-0.9, -0.6, p.z));
    let tail = (1.0 - smoothstep(1.0, 1.3, side)) * (1.0 - smoothstep(-2.0, -1.8, p.z));
    // Se usa la normal del modelo, no la del mundo, para que no dependa de
    // hacia dónde apunta la nave.
    let facing_back = smoothstep(0.3, 0.8, -fragment.object_normal.z);
    let nozzle = (pods + tail).min(1.0) * facing_back;
    if nozzle <= 0.0 {
        return Vector3::zero();
    }

    let flicker = 0.85 + 0.15 * sample(&frame.noise, Vector3::new(p.x * 2.0, p.y * 2.0, frame.time * 9.0));
    let heat = 0.25 + throttle * 0.75;
    let core = color(1.0, 0.9, 0.7);
    mix(ENGINE, core, throttle * 0.35) * (nozzle * heat * flicker * ENGINE_BRIGHTNESS)
}
