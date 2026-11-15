//! Escena de prueba: un sol, un planeta que orbita y rota, y la nave que
//! controla quien juega. Se reemplaza por el sistema solar real.

use std::sync::Arc;

use raylib::math::Vector3;

use super::{Collider, SceneObject, fit_to_radius};
use crate::config::{SHIP_MODEL_PATH, SPHERE_MODEL_PATH};
use crate::math::{Mat4, Quat};
use crate::mesh::{Mesh, Normals, Obj};
use crate::shaders::{Material, color};
use crate::ship::Ship;

const SUN_RADIUS: f32 = 12.0;

const PLANET_RADIUS: f32 = 8.0;
const PLANET_ORBIT_RADIUS: f32 = 90.0;
/// Radianes por segundo alrededor del sol.
const PLANET_ORBIT_SPEED: f32 = 0.03;
/// Radianes por segundo sobre su propio eje.
const PLANET_SPIN_SPEED: f32 = 0.25;

/// Mitad de la envergadura de la nave, en unidades del mundo.
const SHIP_RADIUS: f32 = 1.5;
/// La nave empieza afuera de la órbita, mirando hacia el sol.
const SHIP_START: Vector3 = Vector3 { x: 0.0, y: 6.0, z: 150.0 };

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

    pub fn ship_start(&self) -> Ship {
        // La punta del modelo mira hacia `+z`; media vuelta la deja mirando al sol.
        Ship::new(SHIP_START, Quat::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), std::f32::consts::PI))
    }

    /// Orbita alrededor del sol y gira sobre su propio eje. Se lee de derecha
    /// a izquierda: escala, gira sobre su eje, se aleja del sol y gira
    /// alrededor de él.
    fn planet_model(&self, time: f32) -> Mat4 {
        Mat4::rotation_y(time * PLANET_ORBIT_SPEED)
            * Mat4::translation(Vector3::new(PLANET_ORBIT_RADIUS, 0.0, 0.0))
            * Mat4::rotation_y(time * PLANET_SPIN_SPEED)
            * fit_to_radius(&self.sphere, PLANET_RADIUS)
    }

    fn planet_center(&self, time: f32) -> Vector3 {
        Mat4::rotation_y(time * PLANET_ORBIT_SPEED).transform_point(Vector3::new(PLANET_ORBIT_RADIUS, 0.0, 0.0))
    }

    /// Cuerpos sólidos en el instante `time`.
    pub fn colliders(&self, time: f32) -> Vec<Collider> {
        // Velocidad del planeta por diferencia entre dos instantes cercanos.
        const H: f32 = 0.01;
        let center = self.planet_center(time);
        let velocity = (center - self.planet_center(time - H)) * (1.0 / H);

        vec![
            Collider { center: self.light_position(), radius: SUN_RADIUS, velocity: Vector3::zero() },
            Collider { center, radius: PLANET_RADIUS, velocity },
        ]
    }

    /// Los objetos a dibujar en el instante `time`.
    pub fn objects(&self, time: f32, ship: &Ship) -> Vec<SceneObject> {
        let sun = SceneObject {
            name: "sol",
            mesh: self.sphere.clone(),
            model: fit_to_radius(&self.sphere, SUN_RADIUS),
            material: Material::Emissive { color: color(1.0, 0.75, 0.3) },
        };

        let planet = SceneObject {
            name: "planeta",
            mesh: self.sphere.clone(),
            model: self.planet_model(time),
            material: Material::Lambert { albedo: color(0.25, 0.55, 0.9) },
        };

        let ship = SceneObject {
            name: "nave",
            mesh: self.ship.clone(),
            model: Mat4::translation(ship.position)
                * Mat4::rotation(ship.orientation)
                * fit_to_radius(&self.ship, SHIP_RADIUS),
            material: Material::Ship { throttle: ship.throttle },
        };

        vec![sun, planet, ship]
    }
}
