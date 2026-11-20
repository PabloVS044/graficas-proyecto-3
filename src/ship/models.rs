//! Las naves que se pueden elegir en el menú. Todo lo que cambia entre una y
//! otra está aquí; la física, la cámara y el tamaño en el mundo son iguales
//! para todas.
//!
//! Para agregar una nave: exportarla de Blender como OBJ con normales, ponerla
//! en `assets/` y agregar una entrada a `SHIPS`. Si la punta quedó hacia `-z`,
//! `turn_around: true` la gira. Las toberas y la pintura están en coordenadas
//! del OBJ, ya girado.

use raylib::math::Vector3;

use crate::mesh::{Mesh, Normals, Obj};
use crate::shaders::{Livery, ShipPaint, color};

pub struct ShipSpec {
    pub name: &'static str,
    /// Quién la modeló.
    pub author: &'static str,
    pub description: &'static str,
    pub path: &'static str,
    /// El modelo se exportó con la punta hacia `-z` y hay que girarlo.
    pub turn_around: bool,
    /// De dónde salen las partículas del turbo.
    pub nozzles: &'static [Vector3],
    pub paint: ShipPaint,
}

impl ShipSpec {
    /// Carga el OBJ con sus normales y, si hace falta, lo gira para que la
    /// punta quede hacia `+z`.
    pub fn load(&self) -> Result<Mesh, String> {
        let mesh = Obj::load(self.path)?.to_mesh(Normals::Authored);
        Ok(if self.turn_around { mesh.turned_around() } else { mesh })
    }
}

const fn point(x: f32, y: f32, z: f32) -> Vector3 {
    Vector3 { x, y, z }
}

pub const SHIPS: &[ShipSpec] = &[
    ShipSpec {
        name: "Raptor",
        author: "Pablo",
        description: "Caza de alas en X con canones en las puntas",
        path: "assets/nave_graficas.obj",
        turn_around: false,
        // Las cuatro puntas de las alas y la cola del fuselaje.
        nozzles: &[
            point(10.6, 2.5, -1.5),
            point(-10.6, 2.5, -1.5),
            point(10.4, -2.2, -1.5),
            point(-10.4, -2.2, -1.5),
            point(0.0, 0.0, -2.2),
        ],
        paint: ShipPaint::Classic,
    },
    ShipSpec {
        name: "Comet",
        author: "Jonathan",
        description: "Nave compacta con forma de ave y motores bajo la cola",
        path: "assets/birdship.obj",
        turn_around: false,
        // El bloque de motores, abajo y atrás.
        nozzles: &[
            point(0.99, -1.12, -3.35),
            point(-0.99, -1.12, -3.35),
            point(0.5, -0.6, -3.4),
            point(-0.5, -0.6, -3.4),
            point(0.0, -1.0, -3.4),
        ],
        paint: ShipPaint::Livery(Livery {
            hull: color(0.40, 0.47, 0.56),
            accent: color(0.95, 0.52, 0.14),
            engine: color(0.3, 0.72, 1.0),
            stripe_width: 0.35,
            tips_from: 2.4,
            nozzles_behind: -3.0,
            panel_scale: 2.2,
            wear: 0.3,
        }),
    },
    ShipSpec {
        name: "Trident",
        author: "Ivana",
        description: "Ala delta ligera, de pocos poligonos",
        path: "assets/nave.obj",
        turn_around: false,
        // El motor central, al pie de la aleta de cola, y las dos cápsulas de
        // cada lado.
        nozzles: &[
            point(0.6, 0.55, -7.8),
            point(-0.6, 0.55, -7.8),
            point(2.4, 0.8, -5.7),
            point(-2.4, 0.8, -5.7),
            point(4.9, 1.1, -5.7),
            point(-4.9, 1.1, -5.7),
        ],
        paint: ShipPaint::Livery(Livery {
            hull: color(0.24, 0.25, 0.29),
            accent: color(0.95, 0.78, 0.16),
            engine: color(0.85, 0.32, 1.0),
            stripe_width: 0.6,
            tips_from: 6.0,
            nozzles_behind: -5.2,
            panel_scale: 1.0,
            wear: 0.25,
        }),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ship_loads() {
        for spec in SHIPS {
            let mesh = spec.load().unwrap_or_else(|err| panic!("{}: {err}", spec.name));
            assert!(mesh.triangle_count() > 0, "{} no tiene triángulos", spec.name);
        }
    }

    /// Las toberas deben quedar en la parte trasera del modelo, cerca de su
    /// superficie: si no, las partículas del turbo saldrían de la nada.
    #[test]
    fn nozzles_sit_at_the_back_of_each_ship() {
        for spec in SHIPS {
            let mesh = spec.load().unwrap();
            let (min_z, max_z) = mesh
                .vertices
                .iter()
                .fold((f32::MAX, f32::MIN), |(lo, hi), vertex| (lo.min(vertex.position.z), hi.max(vertex.position.z)));
            assert!(!spec.nozzles.is_empty(), "{} no tiene toberas", spec.name);

            for nozzle in spec.nozzles {
                assert!(nozzle.z < min_z + (max_z - min_z) * 0.35, "{}: tobera {nozzle:?} lejos de la cola", spec.name);
                // Las caras grandes de un modelo de pocos polígonos dejan sus
                // vértices lejos del centro de la cara: se mide en proporción
                // al tamaño de la nave.
                let nearest = mesh.vertices.iter().map(|vertex| (vertex.position - *nozzle).length()).fold(f32::MAX, f32::min);
                assert!(nearest < mesh.bounds.radius * 0.15, "{}: tobera {nozzle:?} lejos del casco", spec.name);
            }
        }
    }

    /// Girar media vuelta conserva el sentido de los triángulos: las caras
    /// siguen mirando hacia afuera y el rasterizador no las descarta.
    #[test]
    fn turning_around_keeps_faces_pointing_out() {
        let mesh = SHIPS[1].load().unwrap().turned_around();
        let outward = mesh
            .vertices
            .chunks_exact(3)
            .filter(|t| {
                let face = (t[1].position - t[0].position).cross(t[2].position - t[0].position);
                face.length() > 1e-9 && face.dot(t[0].normal) > 0.0
            })
            .count();
        assert!(outward as f32 > mesh.triangle_count() as f32 * 0.75);
    }
}
