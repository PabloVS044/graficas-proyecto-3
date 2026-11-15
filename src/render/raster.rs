//! Rasterización con funciones de arista (viene del lab 6).
//!
//! Las aristas se evalúan con enteros, en dieciseisavos de píxel, con la regla
//! "top-left": dos triángulos vecinos nunca dejan huecos ni pintan dos veces el
//! mismo píxel de su arista compartida.

use super::fragment::Fragment;
use super::pipeline::ScreenVertex;

/// Unidades de punto fijo por píxel.
const SUBPIXEL: i64 = 16;

/// Qué pasó con un triángulo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// Da la espalda a la cámara, o es tan pequeño que su área en punto fijo es cero.
    BackFacing,
    Rasterized,
}

/// Coordenadas de pantalla en punto fijo.
#[derive(Clone, Copy)]
struct Point {
    x: i64,
    y: i64,
}

impl Point {
    fn from_screen(vertex: &ScreenVertex) -> Self {
        Point {
            x: (vertex.x * SUBPIXEL as f32).round() as i64,
            y: (vertex.y * SUBPIXEL as f32).round() as i64,
        }
    }

    /// Centro del píxel `(x, y)`.
    fn pixel_center(x: usize, y: usize) -> Self {
        Point { x: x as i64 * SUBPIXEL + SUBPIXEL / 2, y: y as i64 * SUBPIXEL + SUBPIXEL / 2 }
    }
}

/// El doble del área con signo del triángulo `a, b, p`.
#[inline]
fn edge_function(a: Point, b: Point, p: Point) -> i64 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

#[inline]
fn is_top_left(a: Point, b: Point) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dy == 0 && dx > 0) || dy < 0
}

#[inline]
fn inside(edge: i64, top_left: bool) -> bool {
    edge > 0 || (edge == 0 && top_left)
}

/// Agrega a `out` un fragmento por cada píxel cubierto por el triángulo. Los
/// vértices deben estar al frente de la cámara (ya recortados).
pub fn rasterize(vertices: &[ScreenVertex; 3], width: usize, height: usize, out: &mut Vec<Fragment>) -> Outcome {
    // Las mallas son antihorarias vistas desde afuera. En pantalla `y` crece
    // hacia abajo, así que recorriéndolo como 0, 2, 1 una cara visible da área
    // positiva, y la que da la espalda (área negativa) se descarta.
    let (v0, v1, v2) = (&vertices[0], &vertices[2], &vertices[1]);
    let a = Point::from_screen(v0);
    let b = Point::from_screen(v1);
    let c = Point::from_screen(v2);

    let area = edge_function(a, b, c);
    if area <= 0 {
        return Outcome::BackFacing;
    }
    let inverse_area = 1.0 / area as f32;

    let top_left_bc = is_top_left(b, c);
    let top_left_ca = is_top_left(c, a);
    let top_left_ab = is_top_left(a, b);

    // Rectángulo envolvente recortado a la pantalla. `clamp` y no `min`: si el
    // triángulo sale por la izquierda o por arriba el valor es negativo, y
    // `as usize` lo convertiría en uno enorme.
    let pixel = |value: i64| value.div_euclid(SUBPIXEL);
    let min_x = pixel(a.x.min(b.x).min(c.x)).clamp(0, width as i64) as usize;
    let min_y = pixel(a.y.min(b.y).min(c.y)).clamp(0, height as i64) as usize;
    let max_x = (pixel(a.x.max(b.x).max(c.x)) + 1).clamp(0, width as i64) as usize;
    let max_y = (pixel(a.y.max(b.y).max(c.y)) + 1).clamp(0, height as i64) as usize;

    for y in min_y..max_y {
        for x in min_x..max_x {
            let center = Point::pixel_center(x, y);

            let edge_bc = edge_function(b, c, center);
            let edge_ca = edge_function(c, a, center);
            let edge_ab = edge_function(a, b, center);

            if !(inside(edge_bc, top_left_bc) && inside(edge_ca, top_left_ca) && inside(edge_ab, top_left_ab)) {
                continue;
            }

            // Coordenadas baricéntricas en pantalla.
            let w0 = edge_bc as f32 * inverse_area;
            let w1 = edge_ca as f32 * inverse_area;
            let w2 = edge_ab as f32 * inverse_area;

            // Corrección de perspectiva: lo que varía lineal en pantalla es
            // `atributo / w`, no el atributo. Sin esto los patrones se deforman.
            // De paso, `1/w` interpolado en pantalla es la profundidad del fragmento.
            let p0 = w0 * v0.inverse_w;
            let p1 = w1 * v1.inverse_w;
            let p2 = w2 * v2.inverse_w;
            let inverse_w = p0 + p1 + p2;
            let scale = 1.0 / inverse_w;
            let (p0, p1, p2) = (p0 * scale, p1 * scale, p2 * scale);

            out.push(Fragment {
                x,
                y,
                depth: inverse_w,
                object_position: v0.object_position * p0 + v1.object_position * p1 + v2.object_position * p2,
                world_position: v0.world_position * p0 + v1.world_position * p1 + v2.world_position * p2,
                normal: (v0.normal * p0 + v1.normal * p1 + v2.normal * p2).normalize(),
                uv: v0.uv * p0 + v1.uv * p1 + v2.uv * p2,
            });
        }
    }

    Outcome::Rasterized
}
