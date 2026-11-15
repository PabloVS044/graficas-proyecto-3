//! Cargador de archivos OBJ (viene del lab 6).
//!
//! Lee `v` (posición), `vt` (uv), `vn` (normal) y `f` (cara); ignora lo demás.
//! Una cara con más de tres vértices se divide en triángulos en abanico.

use raylib::math::{Vector2, Vector3};

use super::{Mesh, Vertex};

/// Qué normales usar al convertir el OBJ en malla.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Normals {
    /// Las que trae el archivo. Conserva las aristas duras (la nave).
    Authored,
    /// Promedia las de las caras que comparten una posición. Sirve para
    /// superficies curvas exportadas con normales planas (`sphere.obj`).
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Corner {
    position: usize,
    uv: Option<usize>,
    normal: Option<usize>,
}

pub struct Obj {
    positions: Vec<Vector3>,
    uvs: Vec<Vector2>,
    normals: Vec<Vector3>,
    /// Cada triángulo son tres esquinas con índices desde 0.
    triangles: Vec<[Corner; 3]>,
}

impl Obj {
    pub fn load(path: &str) -> Result<Obj, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|err| format!("No se pudo leer {path}: {err}"))?;

        Obj::parse(&text).map_err(|err| format!("{path}: {err}"))
    }

    pub fn parse(text: &str) -> Result<Obj, String> {
        let mut obj = Obj {
            positions: Vec::new(),
            uvs: Vec::new(),
            normals: Vec::new(),
            triangles: Vec::new(),
        };

        for (number, line) in text.lines().enumerate() {
            let number = number + 1;
            let mut parts = line.split_whitespace();

            match parts.next() {
                Some("v") => {
                    let [x, y, z] = parse_floats(parts, number)?;
                    obj.positions.push(Vector3::new(x, y, z));
                }
                Some("vt") => {
                    let [u, v] = parse_floats(parts, number)?;
                    obj.uvs.push(Vector2::new(u, v));
                }
                Some("vn") => {
                    let [x, y, z] = parse_floats(parts, number)?;
                    obj.normals.push(Vector3::new(x, y, z));
                }
                Some("f") => {
                    let face = parts
                        .map(|part| obj.parse_corner(part, number))
                        .collect::<Result<Vec<_>, _>>()?;

                    if face.len() < 3 {
                        return Err(format!("línea {number}: una cara necesita al menos 3 vértices"));
                    }

                    // Abanico: (0, 1, 2), (0, 2, 3), ...
                    for i in 1..face.len() - 1 {
                        obj.triangles.push([face[0], face[i], face[i + 1]]);
                    }
                }
                _ => {}
            }
        }

        Ok(obj)
    }

    /// Lee una esquina de una cara: `v`, `v/vt`, `v//vn` o `v/vt/vn`.
    fn parse_corner(&self, part: &str, number: usize) -> Result<Corner, String> {
        let mut fields = part.split('/');
        let position = fields.next().unwrap_or("");

        let mut optional = |len: usize| -> Result<Option<usize>, String> {
            match fields.next() {
                None | Some("") => Ok(None),
                Some(field) => resolve_index(field, len, number).map(Some),
            }
        };

        let uv = optional(self.uvs.len())?;
        let normal = optional(self.normals.len())?;

        Ok(Corner {
            position: resolve_index(position, self.positions.len(), number)?,
            uv,
            normal,
        })
    }

    #[allow(dead_code, reason = "API pública que todavía no usa la escena de prueba")]
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Convierte el OBJ en una malla lista para el pipeline.
    pub fn to_mesh(&self, normals: Normals) -> Mesh {
        let smooth = match normals {
            Normals::Smooth => Some(self.smooth_normals()),
            Normals::Authored => None,
        };
        let mut vertices = Vec::with_capacity(self.triangles.len() * 3);

        for triangle in &self.triangles {
            let face_normal = self.face_normal(triangle);

            for corner in triangle {
                let normal = match &smooth {
                    Some(smooth) => smooth[corner.position],
                    None => corner.normal.map_or(face_normal, |index| self.normals[index].normalize()),
                };
                vertices.push(Vertex {
                    position: self.positions[corner.position],
                    normal,
                    uv: corner.uv.map_or(Vector2::zero(), |index| self.uvs[index]),
                });
            }
        }

        Mesh::new(vertices)
    }

    fn face_normal(&self, triangle: &[Corner; 3]) -> Vector3 {
        let [a, b, c] = triangle.map(|corner| self.positions[corner.position]);
        (b - a).cross(c - a).normalize()
    }

    /// Promedia, para cada posición, las normales de las caras que la usan. Si
    /// una esquina no trae normal se usa la de su triángulo.
    fn smooth_normals(&self) -> Vec<Vector3> {
        let mut sums = vec![Vector3::zero(); self.positions.len()];

        for triangle in &self.triangles {
            let face_normal = self.face_normal(triangle);

            for corner in triangle {
                let normal = corner.normal.map_or(face_normal, |index| self.normals[index]);
                sums[corner.position] += normal;
            }
        }

        sums.iter().map(|sum| sum.normalize()).collect()
    }
}

fn parse_floats<'a, const N: usize>(
    mut parts: impl Iterator<Item = &'a str>,
    number: usize,
) -> Result<[f32; N], String> {
    let mut values = [0.0; N];

    for value in &mut values {
        let part = parts
            .next()
            .ok_or_else(|| format!("línea {number}: faltan componentes"))?;
        *value = part
            .parse()
            .map_err(|_| format!("línea {number}: \"{part}\" no es un número"))?;
    }

    Ok(values)
}

/// Los índices del OBJ empiezan en 1. Un índice negativo cuenta desde el
/// último elemento leído hasta ese momento (-1 es el último).
fn resolve_index(field: &str, len: usize, number: usize) -> Result<usize, String> {
    let index: i64 = field
        .parse()
        .map_err(|_| format!("línea {number}: \"{field}\" no es un índice"))?;

    let resolved = if index > 0 { index - 1 } else { len as i64 + index };

    if index == 0 || resolved < 0 || resolved >= len as i64 {
        return Err(format!("línea {number}: el índice {index} está fuera de rango"));
    }

    Ok(resolved as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SHIP_MODEL_PATH, SPHERE_MODEL_PATH};

    #[test]
    fn splits_quads_into_two_triangles() {
        let obj = Obj::parse("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n").unwrap();
        assert_eq!(obj.to_mesh(Normals::Smooth).triangle_count(), 2);
    }

    #[test]
    fn rejects_out_of_range_indices() {
        assert!(Obj::parse("v 0 0 0\nf 1 2 3\n").is_err());
    }

    const TWO_FACES: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\n\
                             vn 0 0 1\nvn 1 0 0\n\
                             f 1//1 2//1 3//1\nf 1//2 3//2 4//2\n";

    #[test]
    fn smooth_averages_normals_of_faces_sharing_a_position() {
        let mesh = Obj::parse(TWO_FACES).unwrap().to_mesh(Normals::Smooth);
        let expected = Vector3::new(1.0, 0.0, 1.0).normalize();
        assert!((mesh.vertices[0].normal - expected).length() < 1e-5);
    }

    #[test]
    fn authored_keeps_hard_edges() {
        let mesh = Obj::parse(TWO_FACES).unwrap().to_mesh(Normals::Authored);
        assert!((mesh.vertices[0].normal - Vector3::new(0.0, 0.0, 1.0)).length() < 1e-5);
        assert!((mesh.vertices[3].normal - Vector3::new(1.0, 0.0, 0.0)).length() < 1e-5);
    }

    /// Las caras deben verse desde afuera (antihorarias): el rasterizador
    /// descarta las demás. Si esto falla, la esfera se dibuja al revés.
    #[test]
    fn sphere_faces_point_outward() {
        let obj = Obj::load(SPHERE_MODEL_PATH).unwrap();
        assert_eq!(obj.triangle_count(), 192);

        for triangle in &obj.triangles {
            let [a, b, c] = triangle.map(|corner| obj.positions[corner.position]);
            assert!((b - a).cross(c - a).dot(a + b + c) > 0.0);
        }
    }

    #[test]
    fn ship_loads_completely() {
        let mesh = Obj::load(SHIP_MODEL_PATH).unwrap().to_mesh(Normals::Authored);
        assert_eq!(mesh.triangle_count(), 39_168);
        assert!(mesh.bounds.radius > 0.0);
    }
}
