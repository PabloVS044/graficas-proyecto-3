//! Escena de prueba: un sol, un planeta que orbita y rota, y la
//! nave. Se reemplaza por el sistema solar real.

use std::sync::Arc;

use raylib::math::Vector3;

use super::{SceneObject, fit_to_radius};
use crate::config::{SHIP_MODEL_PATH, SPHERE_MODEL_PATH};
use crate::math::Mat4;
use crate::mesh::{Mesh, Normals, Obj};
use crate::shaders::{Material, color};

const SUN_RADIUS: f32 = 5.0;

const PLANET_RADIUS: f32 = 2.0;
const PLANET_ORBIT_RADIUS: f32 = 18.0;
/// Radianes por segundo alrededor del sol.
const PLANET_ORBIT_SPEED: f32 = 0.15;
/// Radianes por segundo sobre su propio eje.
const PLANET_SPIN_SPEED: f32 = 0.6;

/// Mitad de la envergadura de la nave, en unidades del mundo.
const SHIP_RADIUS: f32 = 1.5;
const SHIP_POSITION: Vector3 = Vector3 { x: 0.0, y: 4.0, z: 26.0 };

pub struct DemoScene {
    sphere: Arc<Mesh>,
    ship: Arc<Mesh>,
}

impl DemoScene {
    pub fn load() -> Result<DemoScene, String> {
        Ok(DemoScene {
            sphere: Arc::new(Obj::load(SPHERE_MODEL_PATH)?.to_mesh(Normals::Smooth)),
            ship: Arc::new(Obj::load(SHIP_MODEL_PATH)?.to_mesh(Normals::Authored)),
        })
    }

    /// Dónde está el sol: es la luz de la escena.
    pub fn light_position(&self) -> Vector3 {
        Vector3::zero()
    }

    /// Los objetos en el instante `time` (segundos de simulación).
    pub fn objects(&self, time: f32) -> Vec<SceneObject> {
        let sun = SceneObject {
            name: "sol",
            mesh: self.sphere.clone(),
            model: fit_to_radius(&self.sphere, SUN_RADIUS),
            material: Material::Emissive { color: color(1.0, 0.75, 0.3) },
        };

        // Se lee de derecha a izquierda: escala, gira sobre su eje, se aleja
        // del sol y gira alrededor de él.
        let planet = SceneObject {
            name: "planeta",
            mesh: self.sphere.clone(),
            model: Mat4::rotation_y(time * PLANET_ORBIT_SPEED)
                * Mat4::translation(Vector3::new(PLANET_ORBIT_RADIUS, 0.0, 0.0))
                * Mat4::rotation_y(time * PLANET_SPIN_SPEED)
                * fit_to_radius(&self.sphere, PLANET_RADIUS),
            material: Material::Lambert { albedo: color(0.25, 0.55, 0.9) },
        };

        let ship = SceneObject {
            name: "nave",
            mesh: self.ship.clone(),
            model: Mat4::translation(SHIP_POSITION)
                * Mat4::rotation_y(time * 0.4)
                * fit_to_radius(&self.ship, SHIP_RADIUS),
            material: Material::Lambert { albedo: color(0.8, 0.8, 0.85) },
        };

        vec![sun, planet, ship]
    }
}
