//! Funciones de ruido compartidas por los shaders (vienen del lab 6).

use noise::{NoiseFn, OpenSimplex};
use raylib::math::Vector3;

/// Ruido OpenSimplex en un punto. Devuelve valores alrededor de 0.
pub fn sample(noise: &OpenSimplex, position: Vector3) -> f32 {
    noise.get([position.x as f64, position.y as f64, position.z as f64]) as f32
}

/// Suma varias octavas de ruido: cada una tiene el doble de frecuencia y la
/// mitad de amplitud. Más octavas dan más detalle, pero cuestan más.
/// Devuelve valores alrededor de 0.5.
pub fn fbm(noise: &OpenSimplex, mut position: Vector3, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut weight = 0.0;

    for _ in 0..octaves {
        value += sample(noise, position) * amplitude;
        weight += amplitude;
        position *= 2.0;
        amplitude *= 0.5;
    }

    value / weight * 0.5 + 0.5
}

/// Ruido celular (Worley): cada celda de una cuadrícula de lado 1 tiene un punto
/// en una posición aleatoria. Devuelve la distancia al punto más cercano (`f1`),
/// al segundo más cercano (`f2`) y la celda del más cercano.
///
/// `f2 - f1` es casi 0 en el borde entre dos regiones, por eso sirve para
/// dibujar grietas y placas; la celda sirve para darle a cada región sus propios
/// números aleatorios.
pub fn worley(p: Vector3, salt: u32) -> (f32, f32, (i32, i32, i32)) {
    let base = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
    let (mut f1, mut f2) = (f32::MAX, f32::MAX);
    let mut nearest = base;

    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let cell = (base.0 + dx, base.1 + dy, base.2 + dz);
                let point = Vector3::new(
                    cell.0 as f32 + cell_random(cell, salt),
                    cell.1 as f32 + cell_random(cell, salt + 1),
                    cell.2 as f32 + cell_random(cell, salt + 2),
                );
                let distance = (p - point).length_sqr();
                if distance < f1 {
                    f2 = f1;
                    f1 = distance;
                    nearest = cell;
                } else if distance < f2 {
                    f2 = distance;
                }
            }
        }
    }

    (f1.sqrt(), f2.sqrt(), nearest)
}

/// Número pseudoaleatorio entre 0 y 1 para una celda. Mismos datos, mismo número.
pub fn cell_random(cell: (i32, i32, i32), salt: u32) -> f32 {
    let mut hash = (cell.0 as u32).wrapping_mul(73_856_093)
        ^ (cell.1 as u32).wrapping_mul(19_349_663)
        ^ (cell.2 as u32).wrapping_mul(83_492_791)
        ^ salt.wrapping_mul(2_654_435_761);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(1_274_126_177);
    hash ^= hash >> 16;
    hash as f32 / u32::MAX as f32
}
