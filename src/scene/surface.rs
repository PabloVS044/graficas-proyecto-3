//! Objetos sobre la superficie de los planetas: dónde va cada uno y cómo se
//! coloca para que siga al planeta mientras orbita y gira.
//!
//! Las posiciones salen de una semilla: puntos al azar sobre la esfera que se
//! quedan solo donde un ruido supera un umbral (así se agrupan en manchas) y
//! donde el terreno del planeta lo permite. Misma semilla, mismas posiciones.
//!
//! Los objetos no se ven desde lejos: brotan del suelo cuando la nave se
//! acerca, primero los que quedan justo debajo de ella.

use std::sync::Arc;

use noise::OpenSimplex;
use raylib::math::Vector3;

use super::planets::PlanetSpec;
use super::{SceneObject, proximity};
use crate::config::NOISE_SEED;
use crate::math::Mat4;
use crate::mesh::{Mesh, shapes};
use crate::shaders::noise_helpers::{cell_random, fbm};
use crate::shaders::{self, Material, color, mix, smoothstep, surface_distance};

/// Cuánto se extienden, en radianes sobre la esfera, los objetos visibles
/// alrededor del punto bajo la nave cuando está pegada a la superficie.
const GROW_REACH: f32 = 0.75;
/// Ancho del borde donde los objetos van creciendo.
const GROW_EDGE: f32 = 0.25;
/// Cuánto se hunde la base, en alturas del objeto, para que no flote donde el
/// suelo se curva.
const SINK: f32 = 0.06;
/// Intentos por objeto pedido antes de rendirse si casi nada pasa los filtros.
const ATTEMPTS_PER_OBJECT: usize = 40;

/// Tipo de objeto.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Prop {
    Tree,
    Rock,
}

/// Dónde puede ir un objeto según el terreno del planeta.
#[derive(Clone, Copy, Debug)]
pub enum Ground {
    /// Selva de Jungla: tierra firme, sin mar ni mesetas.
    Forest,
    /// Placas de basalto de Ignis, lejos de la lava.
    Crust,
}

/// Cuántos objetos de un tipo lleva un planeta y cómo se reparten.
pub struct SurfaceSpec {
    pub prop: Prop,
    pub count: usize,
    pub ground: Ground,
    /// Tamaño de las manchas donde se agrupan: más alto, manchas más pequeñas.
    pub cluster_scale: f32,
    /// Más alto, manchas más escasas.
    pub cluster_threshold: f32,
    /// Alto mínimo y máximo, en unidades del mundo.
    pub size: (f32, f32),
}

/// Un objeto ya ubicado, en el espacio del planeta.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub prop: Prop,
    /// Punto de la esfera de radio 1 donde se apoya.
    pub direction: Vector3,
    /// Giro sobre su propio eje.
    pub yaw: f32,
    pub size: f32,
    /// Número de 0 a 1 para variar forma y color entre copias.
    pub variety: f32,
}

/// Número entre 0 y 1 que depende solo de la semilla, el índice y `salt`.
fn random(seed: u32, index: usize, salt: u32) -> f32 {
    cell_random((index as i32, seed as i32, 17), salt)
}

/// Punto uniforme sobre la esfera de radio 1: la altura `y` uniforme entre -1
/// y 1 y el ángulo alrededor del eje uniforme da la misma densidad en toda la
/// superficie.
fn point_on_sphere(u: f32, v: f32) -> Vector3 {
    let y = u * 2.0 - 1.0;
    let ring = (1.0 - y * y).max(0.0).sqrt();
    let angle = v * std::f32::consts::TAU;
    Vector3::new(ring * angle.cos(), y, ring * angle.sin())
}

fn ground_allows(ground: Ground, terrain: &OpenSimplex, p: Vector3) -> bool {
    match ground {
        Ground::Forest => shaders::is_forest(terrain, p),
        Ground::Crust => shaders::is_solid_crust(terrain, p),
    }
}

/// Las posiciones de todos los objetos de un planeta para una semilla.
pub fn place(specs: &[SurfaceSpec], seed: u32) -> Vec<Placement> {
    // El terreno lo decide el ruido de los shaders, que no cambia con esta semilla.
    let terrain = OpenSimplex::new(NOISE_SEED);
    let mut placements = Vec::new();

    for (kind, spec) in specs.iter().enumerate() {
        let seed = seed.wrapping_add(kind as u32 * 7919);
        let clusters = OpenSimplex::new(seed);
        let mut placed = 0;

        for index in 0..spec.count * ATTEMPTS_PER_OBJECT {
            if placed == spec.count {
                break;
            }
            let direction = point_on_sphere(random(seed, index, 0), random(seed, index, 1));
            let cluster = fbm(&clusters, direction * spec.cluster_scale, 3);
            if cluster < spec.cluster_threshold || !ground_allows(spec.ground, &terrain, direction) {
                continue;
            }

            let (low, high) = spec.size;
            placements.push(Placement {
                prop: spec.prop,
                direction,
                yaw: random(seed, index, 2) * std::f32::consts::TAU,
                size: low + (high - low) * random(seed, index, 3),
                variety: random(seed, index, 4),
            });
            placed += 1;
        }
    }
    placements
}

/// Matriz que para el objeto sobre el planeta: su `+y` sale perpendicular al
/// suelo en `direction`, girado `yaw` sobre ese eje. Va en el espacio del
/// planeta sin escala, así que se multiplica por `planet_frame`.
pub fn surface_frame(direction: Vector3, yaw: f32, radius: f32) -> Mat4 {
    let up = direction.normalize();
    // Cualquier eje que no sea paralelo a `up` sirve para armar la base.
    let helper = if up.y.abs() < 0.9 { Vector3::new(0.0, 1.0, 0.0) } else { Vector3::new(1.0, 0.0, 0.0) };
    let right = helper.cross(up).normalize();
    let forward = right.cross(up);
    let p = up * radius;

    Mat4::from_rows([
        [right.x, up.x, forward.x, p.x],
        [right.y, up.y, forward.y, p.y],
        [right.z, up.z, forward.z, p.z],
        [0.0, 0.0, 0.0, 1.0],
    ]) * Mat4::rotation_y(yaw)
}

/// Mallas de cada tipo de objeto. Un tipo puede tener varias piezas con su
/// propio material y varias formas para que las copias no se vean iguales.
pub struct PropMeshes {
    trunk: Arc<Mesh>,
    crowns: Vec<Arc<Mesh>>,
    rocks: Vec<Arc<Mesh>>,
}

impl PropMeshes {
    pub fn build() -> PropMeshes {
        PropMeshes {
            trunk: Arc::new(shapes::tree_trunk()),
            crowns: (0..3).map(|variant| Arc::new(shapes::tree_crown(variant))).collect(),
            rocks: (0..3).map(|variant| Arc::new(shapes::basalt_rock(variant))).collect(),
        }
    }

    /// Piezas de un objeto con su material.
    fn parts(&self, placement: &Placement) -> Vec<(Arc<Mesh>, Material)> {
        let v = placement.variety;
        let pick = |meshes: &[Arc<Mesh>]| meshes[(v * meshes.len() as f32) as usize % meshes.len()].clone();
        match placement.prop {
            Prop::Rock => vec![(pick(&self.rocks), Material::Basalt)],
            Prop::Tree => {
                let crown = pick(&self.crowns);
                let leaves = mix(color(0.10, 0.36, 0.08), color(0.34, 0.55, 0.10), v);
                vec![
                    (self.trunk.clone(), Material::Lambert { albedo: color(0.40, 0.25, 0.13) }),
                    (crown, Material::Lambert { albedo: leaves }),
                ]
            }
        }
    }
}

/// Cuánto ha brotado un objeto en `direction`: 0 escondido, 1 completo. Crece
/// con la cercanía de la nave y desde el punto bajo ella hacia afuera.
pub fn growth(direction: Vector3, ship_direction: Vector3, near: f32) -> f32 {
    let reach = GROW_REACH * near;
    smoothstep(0.0, GROW_EDGE, reach - surface_distance(direction, ship_direction))
}

/// Agrega a `objects` los objetos de superficie del planeta que se ven.
pub fn push_objects(
    objects: &mut Vec<SceneObject>,
    meshes: &PropMeshes,
    planet: &PlanetSpec,
    placements: &[Placement],
    frame: Mat4,
    ship_position: Vector3,
) {
    let to_ship = ship_position - frame.origin();
    let near = proximity(to_ship.length(), planet.radius);
    if near <= 0.0 || placements.is_empty() {
        return;
    }
    let ship_direction = frame.inverse_rotate_direction(to_ship);

    for placement in placements {
        let grown = growth(placement.direction, ship_direction, near);
        if grown < 0.02 {
            continue;
        }
        let size = placement.size * grown;
        let model = frame
            * surface_frame(placement.direction, placement.yaw, planet.radius - size * SINK)
            * Mat4::scale(size);
        for (mesh, material) in meshes.parts(placement) {
            objects.push(SceneObject { name: planet.name, mesh, model, material });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::planets::PLANETS;

    fn jungle() -> &'static PlanetSpec {
        PLANETS.iter().find(|planet| planet.name == "Jungla").unwrap()
    }

    #[test]
    fn same_seed_gives_same_positions() {
        let a = place(jungle().surface, 3);
        let b = place(jungle().surface, 3);
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(&b).all(|(a, b)| (a.direction - b.direction).length() < 1e-6));
    }

    #[test]
    fn another_seed_moves_them() {
        let a = place(jungle().surface, 3);
        let b = place(jungle().surface, 4);
        let moved = a.iter().zip(&b).filter(|(a, b)| (a.direction - b.direction).length() > 1e-3).count();
        assert!(moved > a.len() / 2);
    }

    #[test]
    fn every_planet_gets_the_objects_it_asks_for() {
        for planet in PLANETS {
            let placements = place(planet.surface, 7);
            let asked: usize = planet.surface.iter().map(|spec| spec.count).sum();
            assert_eq!(placements.len(), asked, "{} no tiene espacio para sus objetos", planet.name);
        }
    }

    fn planet(name: &str) -> &'static PlanetSpec {
        PLANETS.iter().find(|planet| planet.name == name).unwrap()
    }

    #[test]
    fn trees_only_grow_in_the_forest() {
        let terrain = OpenSimplex::new(NOISE_SEED);
        for placement in place(jungle().surface, 7) {
            assert!(shaders::is_forest(&terrain, placement.direction));
        }
    }

    #[test]
    fn rocks_stay_out_of_the_lava() {
        let terrain = OpenSimplex::new(NOISE_SEED);
        let rocks = place(planet("Ignis").surface, 7);
        assert!(!rocks.is_empty());
        for placement in rocks {
            assert!(shaders::is_solid_crust(&terrain, placement.direction));
        }
    }

    /// El `+y` del objeto apunta hacia afuera del planeta y su base queda en la
    /// superficie, en cualquier punto, incluidos los polos.
    #[test]
    fn objects_stand_upright_on_the_surface() {
        for direction in [Vector3::new(0.3, 0.8, -0.5), Vector3::new(0.0, 1.0, 0.0), Vector3::new(0.0, -1.0, 0.0)] {
            let up = direction.normalize();
            let frame = surface_frame(direction, 1.3, 6.0);
            assert!((frame.origin() - up * 6.0).length() < 1e-4);
            assert!((frame.transform_direction(Vector3::new(0.0, 1.0, 0.0)) - up).length() < 1e-4);
        }
    }

    /// Al girar el planeta, el objeto se mueve con él y sigue de pie.
    #[test]
    fn objects_follow_the_planet_as_it_spins() {
        let local = surface_frame(Vector3::new(1.0, 0.0, 0.0), 0.0, 6.0);
        for angle in [0.0, 1.0, 2.5] {
            let planet = Mat4::translation(Vector3::new(50.0, 0.0, 0.0)) * Mat4::rotation_y(angle);
            let world = planet * local;
            let up = world.transform_direction(Vector3::new(0.0, 1.0, 0.0));
            let from_center = (world.origin() - planet.origin()).normalize();
            assert!((up - from_center).length() < 1e-4);
        }
    }

    #[test]
    fn they_grow_only_with_the_ship_near_and_below_it() {
        let below = Vector3::new(0.0, 1.0, 0.0);
        let far_side = Vector3::new(0.0, -1.0, 0.0);
        assert_eq!(growth(below, below, 0.0), 0.0);
        assert_eq!(growth(below, below, 1.0), 1.0);
        assert_eq!(growth(far_side, below, 1.0), 0.0);
    }
}
