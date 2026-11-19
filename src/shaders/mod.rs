//! Fragment shaders. Cada objeto de la escena lleva un `Material`, y `shade`
//! elige el shader según la variante.
//!
//! Para agregar un shader: una variante nueva en `Material` (con sus
//! parámetros), un archivo en esta carpeta con su función `shade`, y una rama
//! en el `match` de abajo.

mod basic;
mod dormant;
mod gas_giant;
mod ice;
mod jungle;
mod props;
mod ship;
mod sun;
mod volcanic;
pub mod noise_helpers;

pub use jungle::{is_forest, is_settlement};
pub use ice::is_on_big_crack;
pub use volcanic::is_solid_crust;

use raylib::math::Vector3;

use crate::render::{Fragment, FrameContext, ObjectContext};

/// Cómo se ve un objeto.
#[derive(Clone, Copy, Debug)]
pub enum Material {
    /// Difuso (Lambert) iluminado por el sol.
    Lambert { albedo: Vector3 },
    /// Plasma del sol: hierve, tiene manchas y filamentos que se encienden con
    /// la nave cerca. Emite su propio color.
    Sun,
    /// Planeta de selva con mar, ríos, roca, nubes con sombra, reflejo en el
    /// agua y luces de noche.
    Jungle,
    /// Corteza de basalto partida en placas con lava en las grietas y volcanes.
    /// Al acercarse la nave, las grietas se abren y salen ondas de calor.
    Volcanic,
    /// Gigante gaseoso con bandas que giran a distinta velocidad, una tormenta
    /// en espiral y relámpagos. La nave cerca agita la tormenta.
    GasGiant,
    /// Hielo con grietas y auroras. La nave cerca enciende la bioluminiscencia
    /// de las grietas en ondas que salen de debajo de ella.
    Ice,
    /// Roca con una red hexagonal tallada que se enciende, hexágono por
    /// hexágono, desde el punto bajo la nave.
    Dormant,
    /// Roca de basalto con lava encendida en los poros.
    Basalt,
    /// Cristal de hielo que brilla por dentro; `glow` de 0 a 1 según la cercanía
    /// de la nave a su planeta.
    Crystal { glow: f32 },
    /// Copa de árbol con luciérnagas de noche. `ground_up`: el arriba del suelo
    /// donde está, en el mundo.
    Foliage { albedo: Vector3, ground_up: Vector3 },
    /// Techo de paja de una choza.
    Thatch,
    /// Pared de barro de una choza, con el fuego encendido en la puerta.
    HutWall { ground_up: Vector3 },
    /// Casco desgastado con franjas rojas y motores encendidos.
    /// `throttle` va de 0 (reposo) a 1 (turbo); `heat`, de 0 (frío) a 1
    /// (al rojo vivo, junto al sol o a un planeta volcánico).
    Ship { throttle: f32, heat: f32 },
    /// Pinta la normal como color. Sirve para revisar geometría.
    #[allow(dead_code, reason = "API pública que todavía no usa la escena de prueba")]
    Normals,
}

pub fn shade(material: &Material, fragment: &Fragment, frame: &FrameContext, object: &ObjectContext) -> Vector3 {
    match *material {
        Material::Lambert { albedo } => basic::lambert(albedo, fragment, frame),
        Material::Sun => sun::shade(fragment, frame, object),
        Material::Ship { throttle, heat } => ship::shade(fragment, frame, throttle, heat),
        Material::Jungle => jungle::shade(fragment, frame, object),
        Material::Volcanic => volcanic::shade(fragment, frame, object),
        Material::GasGiant => gas_giant::shade(fragment, frame, object),
        Material::Ice => ice::shade(fragment, frame, object),
        Material::Dormant => dormant::shade(fragment, frame, object),
        Material::Basalt => props::basalt(fragment, frame),
        Material::Foliage { albedo, ground_up } => props::foliage(fragment, frame, albedo, ground_up),
        Material::Thatch => props::thatch(fragment, frame),
        Material::HutWall { ground_up } => props::hut_wall(fragment, frame, ground_up),
        Material::Crystal { glow } => props::crystal(fragment, frame, glow),
        Material::Normals => basic::normals(fragment),
    }
}

/// Atajo para escribir un color con tres componentes.
pub const fn color(r: f32, g: f32, b: f32) -> Vector3 {
    Vector3 { x: r, y: g, z: b }
}

/// Interpolación lineal entre dos colores.
pub fn mix(a: Vector3, b: Vector3, amount: f32) -> Vector3 {
    a * (1.0 - amount) + b * amount
}

/// 0 antes de `start`, 1 después de `end` y una transición suave entre ambos.
pub fn smoothstep(start: f32, end: f32, value: f32) -> f32 {
    let t = ((value - start) / (end - start)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Distancia sobre la superficie entre dos puntos de una esfera de radio 1, en
/// radianes: 0 en el mismo punto, π en el opuesto.
pub fn surface_distance(a: Vector3, b: Vector3) -> f32 {
    a.dot(b).clamp(-1.0, 1.0).acos()
}

/// Dirección normalizada del fragmento hacia el sol.
pub fn light_direction(fragment: &Fragment, frame: &FrameContext) -> Vector3 {
    (frame.light_position - fragment.world_position).normalize()
}
