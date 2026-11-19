//! El sistema solar: dónde está cada cuerpo en cada instante, qué se dibuja y
//! contra qué choca la nave. Los datos de cada cuerpo están en `planets.rs`.

use std::sync::Arc;

use raylib::math::Vector3;

use super::planets::{MoonSpec, PLANETS, PlanetSpec, SUN};
use super::{Collider, SceneObject, fit_to_radius, proximity};
use crate::config::{RENDER_HEIGHT, SHIP_MODEL_PATH};
use crate::render::Camera;
use crate::math::{Mat4, Quat};
use crate::mesh::{Mesh, Normals, Obj, icosphere};
use crate::shaders::Material;
use crate::ship::Ship;

/// Mitad de la envergadura de la nave, en unidades del mundo.
const SHIP_RADIUS: f32 = 1.5;
/// La nave empieza entre la segunda y la tercera órbita, mirando al sol.
const SHIP_START: Vector3 = Vector3 { x: 0.0, y: 10.0, z: 122.0 };

/// Niveles de subdivisión de las esferas: de 80 a 20 480 triángulos.
const MIN_SPHERE_LEVEL: u32 = 1;
pub const MAX_SPHERE_LEVEL: u32 = 5;
/// Largo deseado, en píxeles, del lado de un triángulo de una esfera.
const TARGET_EDGE_PIXELS: f32 = 6.0;
/// Largo del lado de un icosaedro inscrito en la esfera de radio 1.
const ICOSAHEDRON_EDGE: f32 = 1.051;

/// Cómo se elige el nivel de detalle de las esferas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Detail {
    /// Según el tamaño en pantalla.
    ByDistance,
    /// Siempre el máximo. Sirve para comparar cuánto trabajo se ahorra.
    Max,
}

pub struct SolarSystem {
    /// Una icosfera por nivel de subdivisión, de 0 a `MAX_SPHERE_LEVEL`.
    spheres: Vec<Arc<Mesh>>,
    ship: Arc<Mesh>,
}

/// Posición en una órbita circular: gira en su plano y después se inclina.
fn orbit_position(radius: f32, phase: f32, speed: f32, tilt: f32, time: f32) -> Vector3 {
    (Mat4::rotation_x(tilt) * Mat4::rotation_y(phase + speed * time)).transform_point(Vector3::new(radius, 0.0, 0.0))
}

fn moon_offset(moon: &MoonSpec, time: f32) -> Vector3 {
    orbit_position(moon.orbit_radius, moon.orbit_phase, moon.orbit_speed, moon.orbit_tilt, time)
}

impl SolarSystem {
    pub fn load() -> Result<SolarSystem, String> {
        Ok(SolarSystem {
            spheres: (0..=MAX_SPHERE_LEVEL).map(|level| Arc::new(icosphere(level))).collect(),
            ship: Arc::new(Obj::load(SHIP_MODEL_PATH)?.to_mesh(Normals::Authored)),
        })
    }

    /// El sol está en el origen y es la luz de la escena.
    pub fn light_position(&self) -> Vector3 {
        Vector3::zero()
    }

    pub fn ship_start(&self) -> Ship {
        // La punta del modelo mira hacia `+z`; media vuelta la deja mirando al sol.
        Ship::new(SHIP_START, Quat::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), std::f32::consts::PI))
    }

    pub fn planet_center(&self, planet: &PlanetSpec, time: f32) -> Vector3 {
        orbit_position(planet.orbit_radius, planet.orbit_phase, planet.orbit_speed, planet.orbit_tilt, time)
    }

    /// Marco del planeta sin escala: su centro, la inclinación de su eje y su
    /// giro. Un objeto sobre la superficie se coloca multiplicando por esta
    /// matriz, y así orbita y gira con el planeta.
    pub fn planet_frame(&self, planet: &PlanetSpec, time: f32) -> Mat4 {
        Mat4::translation(self.planet_center(planet, time))
            * Mat4::rotation_z(planet.axial_tilt)
            * Mat4::rotation_y(planet.spin_speed * time)
    }

    /// El planeta al que la nave está más cerca, con su índice en `PLANETS` y su
    /// `proximity`. `None` si la nave no está cerca de ninguno.
    pub fn nearest_planet(&self, time: f32, position: Vector3) -> Option<(usize, f32)> {
        PLANETS
            .iter()
            .enumerate()
            .map(|(index, planet)| {
                let distance = (position - self.planet_center(planet, time)).length();
                (index, proximity(distance, planet.radius))
            })
            .filter(|(_, near)| *near > 0.0)
            .max_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// `proximity` de la nave al sol.
    pub fn sun_proximity(&self, position: Vector3) -> f32 {
        proximity((position - self.light_position()).length(), SUN.radius)
    }

    /// Cuánto se calienta el casco: lo cerca que esté del sol o de un planeta
    /// volcánico, de 0 a 1.
    pub fn ship_heat(&self, time: f32, position: Vector3) -> f32 {
        PLANETS
            .iter()
            .filter(|planet| matches!(planet.material, Material::Volcanic))
            .map(|planet| proximity((position - self.planet_center(planet, time)).length(), planet.radius))
            .fold(self.sun_proximity(position), f32::max)
    }

    /// Cuerpos sólidos en el instante `time`, con su velocidad.
    pub fn colliders(&self, time: f32) -> Vec<Collider> {
        // Velocidad por diferencia entre dos instantes cercanos.
        const H: f32 = 0.01;
        let collider = |radius: f32, center: &dyn Fn(f32) -> Vector3| Collider {
            center: center(time),
            radius,
            velocity: (center(time) - center(time - H)) * (1.0 / H),
        };

        let mut colliders = vec![Collider { center: self.light_position(), radius: SUN.radius, velocity: Vector3::zero() }];
        for planet in PLANETS {
            colliders.push(collider(planet.radius, &|t| self.planet_center(planet, t)));
            for moon in planet.moons {
                colliders.push(collider(moon.radius, &|t| self.planet_center(planet, t) + moon_offset(moon, t)));
            }
        }
        colliders
    }

    /// Nivel de subdivisión para una esfera de `radius` con centro en `center`
    /// vista desde `camera`. Se elige para que los lados de los triángulos midan
    /// en pantalla como mucho `TARGET_EDGE_PIXELS`: un planeta lejano que ocupa
    /// pocos píxeles no necesita miles de triángulos.
    pub fn sphere_level(&self, center: Vector3, radius: f32, camera: &Camera, detail: Detail) -> u32 {
        if detail == Detail::Max {
            return MAX_SPHERE_LEVEL;
        }
        let distance = (center - camera.position).length();
        if distance <= radius {
            return MAX_SPHERE_LEVEL;
        }

        // Radio aparente en píxeles, y de ahí cuánto mide un lado del icosaedro.
        let pixels_per_unit = RENDER_HEIGHT as f32 * 0.5 / (camera.field_of_view * 0.5).tan();
        let radius_pixels = radius / distance * pixels_per_unit;
        let edge_pixels = radius_pixels * ICOSAHEDRON_EDGE;
        // Cada nivel parte los lados a la mitad.
        let level = (edge_pixels / TARGET_EDGE_PIXELS).log2().ceil();
        level.clamp(MIN_SPHERE_LEVEL as f32, MAX_SPHERE_LEVEL as f32) as u32
    }

    /// Los objetos a dibujar en el instante `time`, vistos desde `camera`.
    pub fn objects(&self, time: f32, ship: &Ship, camera: &Camera, detail: Detail) -> Vec<SceneObject> {
        // `frame` coloca y orienta la esfera sin escala. La icosfera tiene radio 1
        // y está centrada, así que basta con escalarla al radio del cuerpo.
        let sphere = |name: &'static str, frame: Mat4, radius: f32, material: Material| SceneObject {
            name,
            mesh: self.spheres[self.sphere_level(frame.origin(), radius, camera, detail) as usize].clone(),
            model: frame * Mat4::scale(radius),
            material,
        };

        let mut objects = vec![sphere("Sol", Mat4::translation(self.light_position()), SUN.radius, SUN.material)];

        for planet in PLANETS {
            objects.push(sphere(planet.name, self.planet_frame(planet, time), planet.radius, planet.material));

            let center = self.planet_center(planet, time);
            for moon in planet.moons {
                // Gira lo mismo que avanza en su órbita: siempre muestra la misma cara.
                let frame = Mat4::translation(center + moon_offset(moon, time))
                    * Mat4::rotation_x(moon.orbit_tilt)
                    * Mat4::rotation_y(moon.orbit_phase + moon.orbit_speed * time);
                objects.push(sphere(moon.name, frame, moon.radius, moon.material));
            }
        }

        objects.push(SceneObject {
            name: "Nave",
            mesh: self.ship.clone(),
            model: Mat4::translation(ship.position) * Mat4::rotation(ship.orientation) * fit_to_radius(&self.ship, SHIP_RADIUS),
            material: Material::Ship { throttle: ship.throttle, heat: self.ship_heat(time, ship.position) },
        });

        objects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ship::COLLISION_RADIUS;

    /// Lo más lejos que llega un planeta con sus lunas, desde su centro.
    fn reach(planet: &PlanetSpec) -> f32 {
        planet.moons.iter().map(|moon| moon.orbit_radius + moon.radius).fold(planet.radius, f32::max)
    }

    /// Entre dos órbitas vecinas debe caber la nave, aunque los planetas
    /// estén alineados.
    #[test]
    fn orbits_do_not_overlap() {
        let mut inner = SUN.radius;
        for planet in PLANETS {
            let gap = planet.orbit_radius - reach(planet) - inner;
            assert!(gap > COLLISION_RADIUS * 4.0, "{} queda demasiado cerca de la órbita anterior", planet.name);
            inner = planet.orbit_radius + reach(planet);
        }
    }

    #[test]
    fn moons_do_not_touch_their_planet() {
        for planet in PLANETS {
            for moon in planet.moons {
                let gap = moon.orbit_radius - moon.radius - planet.radius;
                assert!(gap > COLLISION_RADIUS * 4.0, "{} roza a {}", moon.name, planet.name);
            }
        }
    }

    /// Todos orbitan y giran, y la posición depende del tiempo: después de
    /// una vuelta completa el planeta vuelve al mismo lugar.
    #[test]
    fn planets_orbit_and_spin_with_time() {
        let system = SolarSystem::load().unwrap();
        for planet in PLANETS {
            assert!(planet.orbit_speed > 0.0 && planet.spin_speed > 0.0, "{} está quieto", planet.name);

            let period = std::f32::consts::TAU / planet.orbit_speed;
            let start = system.planet_center(planet, 3.0);
            assert!((system.planet_center(planet, 3.0 + period) - start).length() < 0.05);
            assert!((system.planet_center(planet, 3.0 + period * 0.5) - start).length() > planet.orbit_radius);
        }
    }

    #[test]
    fn ship_starts_outside_every_body() {
        let system = SolarSystem::load().unwrap();
        let ship = system.ship_start();
        for collider in system.colliders(0.0) {
            assert!((ship.position - collider.center).length() > collider.radius + COLLISION_RADIUS * 4.0);
        }
    }

    #[test]
    fn every_body_is_drawn_and_solid() {
        let system = SolarSystem::load().unwrap();
        let bodies = 1 + PLANETS.len() + PLANETS.iter().map(|planet| planet.moons.len()).sum::<usize>();
        assert_eq!(system.colliders(0.0).len(), bodies);
        let camera = camera_at(Vector3::new(0.0, 0.0, 300.0));
        assert_eq!(system.objects(0.0, &system.ship_start(), &camera, Detail::ByDistance).len(), bodies + 1);
    }

    fn camera_at(position: Vector3) -> Camera {
        Camera {
            position,
            target: Vector3::zero(),
            up: Vector3::new(0.0, 1.0, 0.0),
            field_of_view: 60f32.to_radians(),
            near: 0.1,
            far: 1000.0,
        }
    }

    #[test]
    fn farther_spheres_get_fewer_triangles() {
        let system = SolarSystem::load().unwrap();
        let level = |distance: f32| system.sphere_level(Vector3::zero(), 6.0, &camera_at(Vector3::new(0.0, 0.0, distance)), Detail::ByDistance);

        assert_eq!(level(7.0), MAX_SPHERE_LEVEL, "de cerca, el máximo");
        assert_eq!(level(900.0), MIN_SPHERE_LEVEL, "muy lejos, el mínimo");
        let levels: Vec<u32> = [10.0, 30.0, 80.0, 200.0, 500.0].map(level).to_vec();
        assert!(levels.windows(2).all(|pair| pair[0] >= pair[1]), "{levels:?}");
    }

    #[test]
    fn max_detail_ignores_distance() {
        let system = SolarSystem::load().unwrap();
        let far = camera_at(Vector3::new(0.0, 0.0, 900.0));
        assert_eq!(system.sphere_level(Vector3::zero(), 6.0, &far, Detail::Max), MAX_SPHERE_LEVEL);
    }
}
