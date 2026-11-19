//! Los cuerpos del sistema solar. Todo lo que define a cada uno está aquí.
//!
//! Para cambiar un planeta, se edita su entrada. Para agregar uno, se agrega
//! una entrada a `PLANETS` con una órbita que no choque con las vecinas (la
//! prueba `orbits_do_not_overlap` lo revisa). Para darle apariencia propia, se
//! crea su `Material` en `src/shaders/` y se pone aquí.
//!
//! Unidades: distancias en unidades del mundo, ángulos en radianes,
//! velocidades en radianes por segundo de simulación.

use super::surface::{Ground, Prop, SurfaceSpec};
use crate::shaders::{Material, color};

pub struct SunSpec {
    pub radius: f32,
    pub material: Material,
}

pub struct PlanetSpec {
    pub name: &'static str,
    /// Frase que aparece en pantalla al acercarse. Sin acentos: la fuente del
    /// HUD no los tiene.
    pub description: &'static str,
    pub radius: f32,
    /// Distancia al sol.
    pub orbit_radius: f32,
    /// Qué tan rápido da la vuelta al sol.
    pub orbit_speed: f32,
    /// Dónde empieza su órbita, para que los planetas no salgan alineados.
    pub orbit_phase: f32,
    /// Inclinación del plano de su órbita respecto al de los demás.
    pub orbit_tilt: f32,
    /// Qué tan rápido gira sobre su propio eje.
    pub spin_speed: f32,
    /// Inclinación de su eje de rotación.
    pub axial_tilt: f32,
    pub material: Material,
    pub moons: &'static [MoonSpec],
    /// Objetos sobre su superficie. Ver `surface.rs`.
    pub surface: &'static [SurfaceSpec],
}

/// Una luna orbita alrededor de su planeta y siempre le muestra la misma cara.
pub struct MoonSpec {
    pub name: &'static str,
    pub radius: f32,
    /// Distancia al centro de su planeta.
    pub orbit_radius: f32,
    pub orbit_speed: f32,
    pub orbit_phase: f32,
    pub orbit_tilt: f32,
    pub material: Material,
}

pub const SUN: SunSpec = SunSpec { radius: 12.0, material: Material::Sun };

/// Ordenados del más cercano al sol al más lejano.
pub const PLANETS: &[PlanetSpec] = &[
    PlanetSpec {
        name: "Jungla",
        description: "Selva viva bajo las nubes. Las nubes se abren al pasar.",
        radius: 6.0,
        orbit_radius: 60.0,
        orbit_speed: 0.05,
        orbit_phase: 0.4,
        orbit_tilt: 0.02,
        spin_speed: 0.3,
        axial_tilt: 0.3,
        material: Material::Jungle,
        moons: &[],
        surface: &[SurfaceSpec {
            prop: Prop::Tree,
            count: 450,
            ground: Ground::Forest,
            cluster_scale: 3.0,
            cluster_threshold: 0.47,
            size: (0.18, 0.32),
        }],
    },
    PlanetSpec {
        name: "Ambar",
        description: "Gigante gaseoso. Su tormenta despierta con visitas.",
        radius: 9.0,
        orbit_radius: 100.0,
        orbit_speed: 0.035,
        orbit_phase: 2.2,
        orbit_tilt: -0.03,
        spin_speed: 0.2,
        axial_tilt: 0.1,
        material: Material::GasGiant,
        moons: &[MoonSpec {
            name: "Luna de Ambar",
            radius: 1.6,
            orbit_radius: 17.0,
            orbit_speed: 0.4,
            orbit_phase: 1.0,
            orbit_tilt: 0.35,
            material: Material::Lambert { albedo: color(0.65, 0.65, 0.65) },
        }],
        surface: &[],
    },
    PlanetSpec {
        name: "Kryos",
        description: "Oceano congelado. Algo brilla bajo el hielo.",
        radius: 5.0,
        orbit_radius: 145.0,
        orbit_speed: 0.025,
        orbit_phase: 4.0,
        orbit_tilt: 0.04,
        spin_speed: 0.35,
        axial_tilt: 0.5,
        material: Material::Ice,
        moons: &[],
        surface: &[],
    },
    PlanetSpec {
        name: "Ignis",
        description: "Mundo volcanico. El casco se calienta.",
        radius: 7.0,
        orbit_radius: 190.0,
        orbit_speed: 0.02,
        orbit_phase: 5.3,
        orbit_tilt: -0.02,
        spin_speed: 0.25,
        axial_tilt: 0.2,
        material: Material::Volcanic,
        moons: &[],
        surface: &[],
    },
    PlanetSpec {
        name: "Eco",
        description: "Planeta dormido. Parece reconocer a la nave.",
        radius: 4.5,
        orbit_radius: 240.0,
        orbit_speed: 0.015,
        orbit_phase: 1.2,
        orbit_tilt: 0.05,
        spin_speed: 0.4,
        axial_tilt: 0.15,
        material: Material::Dormant,
        moons: &[],
        surface: &[],
    },
];
