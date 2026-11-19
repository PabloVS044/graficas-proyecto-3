//! Modelos de los objetos de superficie, generados en código. Se dibujan con
//! caras planas (una normal por triángulo) para que se vean facetados.
//!
//! Cada modelo tiene la base en el origen y crece hacia `+y`, con 1 de alto
//! aproximadamente: la escena lo escala y lo apoya sobre el planeta.

use std::f32::consts::TAU;

use noise::OpenSimplex;
use raylib::math::{Vector2, Vector3};

use super::{Mesh, Vertex, icosphere};
use crate::math::{Mat4, Quat};
use crate::shaders::noise_helpers::{cell_random, sample, worley};
use crate::shaders::smoothstep;

/// Acumula triángulos con su normal de cara.
#[derive(Default)]
struct Builder {
    vertices: Vec<Vertex>,
}

impl Builder {
    /// `a`, `b`, `c` en orden antihorario visto desde afuera.
    fn triangle(&mut self, a: Vector3, b: Vector3, c: Vector3) {
        self.tagged_triangle([a, b, c], [0.0; 3]);
    }

    /// Como `triangle`, con un valor por esquina guardado en `uv.x` para que el
    /// shader lo reciba interpolado.
    fn tagged_triangle(&mut self, corners: [Vector3; 3], tags: [f32; 3]) {
        let [a, b, c] = corners;
        let normal = (b - a).cross(c - a).normalize();
        for (position, tag) in corners.into_iter().zip(tags) {
            self.vertices.push(Vertex { position, normal, uv: Vector2::new(tag, 0.0) });
        }
    }

    fn build(self) -> Mesh {
        Mesh::new(self.vertices)
    }
}

/// Anillo de `sides` puntos a la altura `y`, desplazado en `x` por `lean`.
fn ring(sides: usize, radius: f32, y: f32, lean: f32, twist: f32) -> Vec<Vector3> {
    (0..sides)
        .map(|i| {
            let angle = i as f32 / sides as f32 * TAU + twist;
            Vector3::new(angle.cos() * radius + lean, y, -angle.sin() * radius)
        })
        .collect()
}

/// Une dos anillos con el mismo número de puntos con una franja de triángulos.
fn band(builder: &mut Builder, lower: &[Vector3], upper: &[Vector3]) {
    let n = lower.len();
    for i in 0..n {
        let j = (i + 1) % n;
        builder.triangle(lower[i], lower[j], upper[j]);
        builder.triangle(lower[i], upper[j], upper[i]);
    }
}

/// Esfera irregular: una icosfera cuyos puntos se empujan hacia afuera o hacia
/// adentro con ruido. El ruido depende solo de la posición, así que los
/// triángulos vecinos siguen compartiendo sus esquinas y no se abren huecos.
fn lumpy_sphere(level: u32, seed: u32, frequency: f32, amount: f32, scale: Vector3, center: Vector3) -> Builder {
    let noise = OpenSimplex::new(seed);
    displaced_sphere(level, scale, center, |p| (1.0 + sample(&noise, p * frequency) * amount, 0.0))
}

/// Icosfera deformada por `shape`, que devuelve para cada punto de la esfera
/// de radio 1 cuánto se aleja del centro y un valor para el shader.
fn displaced_sphere(level: u32, scale: Vector3, center: Vector3, shape: impl Fn(Vector3) -> (f32, f32)) -> Builder {
    let place = |p: Vector3| {
        let (distance, tag) = shape(p);
        let p = p * distance;
        (Vector3::new(p.x * scale.x, p.y * scale.y, p.z * scale.z) + center, tag)
    };

    let mut builder = Builder::default();
    for t in icosphere(level).vertices.chunks_exact(3) {
        let [(a, ta), (b, tb), (c, tc)] = [place(t[0].position), place(t[1].position), place(t[2].position)];
        builder.tagged_triangle([a, b, c], [ta, tb, tc]);
    }
    builder
}

/// Tronco de árbol: un prisma de seis lados que se adelgaza y se inclina un poco.
pub fn tree_trunk() -> Mesh {
    let mut builder = Builder::default();
    let rings = [
        ring(6, 0.07, -0.08, 0.0, 0.0),
        ring(6, 0.055, 0.30, 0.02, 0.2),
        ring(6, 0.04, 0.62, 0.05, 0.4),
    ];
    band(&mut builder, &rings[0], &rings[1]);
    band(&mut builder, &rings[1], &rings[2]);
    builder.build()
}

/// Copa de árbol: dos bultos de follaje irregulares encima del tronco.
/// `variant` cambia la forma para que no todos los árboles sean iguales.
pub fn tree_crown(variant: u32) -> Mesh {
    let seed = 100 + variant;
    let mut crown = lumpy_sphere(1, seed, 1.6, 0.28, Vector3::new(0.30, 0.22, 0.30), Vector3::new(0.05, 0.70, 0.0));
    let side = lumpy_sphere(1, seed + 50, 1.9, 0.3, Vector3::new(0.19, 0.15, 0.19), Vector3::new(-0.14, 0.60, 0.10));
    crown.vertices.extend(side.vertices);
    crown.build()
}

/// Peña de basalto: más alta que ancha, irregular y con hendiduras. Cada
/// hendidura es una celda de Worley hundida; su profundidad (0 a 1) va en
/// `uv.x` para que el shader encienda la lava dentro.
pub fn basalt_rock(variant: u32) -> Mesh {
    let seed = 300 + variant;
    let noise = OpenSimplex::new(seed);
    let builder = displaced_sphere(2, Vector3::new(0.30, 0.72, 0.27), Vector3::new(0.0, 0.50, 0.0), |p| {
        // Ancha abajo y en punta arriba, con bultos grandes y aristas.
        let taper = 1.0 - (p.y * 0.5 + 0.5).powf(1.5) * 0.6;
        let bumps = 1.0 + sample(&noise, p * 1.9) * 0.35 + sample(&noise, p * 5.0) * 0.1;
        let (f1, _, _) = worley(p * 2.2 + Vector3::new(variant as f32 * 5.0, 0.0, 0.0), seed);
        let pore = 1.0 - smoothstep(0.1, 0.45, f1);
        (taper * bumps - pore * 0.22, pore)
    });
    builder.build()
}

/// Prisma hexagonal con punta, de pie sobre el origen. La altura relativa de
/// cada vértice (0 en la base, 1 en la punta) va en `uv.x`.
fn crystal_prism(builder: &mut Builder, radius: f32, height: f32, tilt: Mat4) {
    let body = height * 0.78;
    let lower = ring(6, radius, -0.05, 0.0, 0.0);
    let upper = ring(6, radius * 0.9, body, 0.0, 0.0);
    let tip = Vector3::new(0.0, height, 0.0);
    let place = |p: Vector3| tilt.transform_point(p);
    let tag = |p: Vector3| (p.y / height).clamp(0.0, 1.0);

    for i in 0..6 {
        let j = (i + 1) % 6;
        let (a, b, c, d) = (lower[i], lower[j], upper[j], upper[i]);
        builder.tagged_triangle([a, b, c].map(place), [tag(a), tag(b), tag(c)]);
        builder.tagged_triangle([a, c, d].map(place), [tag(a), tag(c), tag(d)]);
        builder.tagged_triangle([d, c, tip].map(place), [tag(d), tag(c), 1.0]);
    }
}

/// Grupo de cristales de hielo: entre tres y cinco prismas que salen del mismo
/// punto, cada uno inclinado hacia afuera.
pub fn ice_crystals(variant: u32) -> Mesh {
    let random = |salt: u32| cell_random((variant as i32, 5, 9), salt);
    let mut builder = Builder::default();
    let count = 3 + (random(0) * 3.0) as u32;

    for k in 0..count {
        let angle = k as f32 / count as f32 * TAU + random(10 + k) * 0.8;
        let lean = if k == 0 { 0.05 } else { 0.3 + random(20 + k) * 0.35 };
        let axis = Vector3::new(angle.cos(), 0.0, -angle.sin()).cross(Vector3::new(0.0, 1.0, 0.0));
        let height = if k == 0 { 1.0 } else { 0.45 + random(30 + k) * 0.4 };
        let tilt = Mat4::rotation(Quat::from_axis_angle(axis.normalize(), lean));
        crystal_prism(&mut builder, 0.09 + random(40 + k) * 0.04, height, tilt);
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meshes() -> Vec<Mesh> {
        vec![tree_trunk(), tree_crown(0), tree_crown(1), basalt_rock(0), basalt_rock(1), ice_crystals(0), ice_crystals(1)]
    }

    /// Sin triángulos degenerados: una normal con largo 1 en cada vértice.
    #[test]
    fn normals_are_unit_length() {
        for mesh in meshes() {
            assert!(mesh.vertices.iter().all(|v| (v.normal.length() - 1.0).abs() < 1e-3));
        }
    }

    /// Volumen con signo de una malla cerrada: positivo si las caras miran hacia afuera.
    fn signed_volume(mesh: &Mesh) -> f32 {
        mesh.vertices
            .chunks_exact(3)
            .map(|t| t[0].position.dot(t[1].position.cross(t[2].position)) / 6.0)
            .sum()
    }

    #[test]
    fn crowns_face_outwards() {
        for variant in 0..3 {
            assert!(signed_volume(&tree_crown(variant)) > 0.0);
            assert!(signed_volume(&basalt_rock(variant)) > 0.0);
            assert!(signed_volume(&ice_crystals(variant)) > 0.0);
        }
    }

    /// El tronco es un tubo abierto: sus caras miran lejos del eje vertical.
    #[test]
    fn trunk_faces_outwards() {
        for t in tree_trunk().vertices.chunks_exact(3) {
            let centroid = (t[0].position + t[1].position + t[2].position) * (1.0 / 3.0);
            // El eje se inclina: a cada altura su centro está un poco hacia `+x`.
            let lean = if centroid.y < 0.30 { (centroid.y + 0.08) / 0.38 * 0.02 } else { 0.02 + (centroid.y - 0.30) / 0.32 * 0.03 };
            let outward = Vector3::new(centroid.x - lean, 0.0, centroid.z);
            assert!(t[0].normal.dot(outward) > 0.0, "triángulo al revés");
        }
    }

    #[test]
    fn rocks_have_pores() {
        let rock = basalt_rock(0);
        let deep = rock.vertices.iter().filter(|v| v.uv.x > 0.5).count();
        assert!(deep > 0 && deep < rock.vertices.len() / 2);
    }

    #[test]
    fn models_stand_on_the_origin_and_are_about_one_tall() {
        let top = tree_crown(0).vertices.iter().map(|v| v.position.y).fold(f32::MIN, f32::max);
        let bottom = tree_trunk().vertices.iter().map(|v| v.position.y).fold(f32::MAX, f32::min);
        assert!(bottom < 0.0 && bottom > -0.2);
        assert!(top > 0.8 && top < 1.2);
    }
}
