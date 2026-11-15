use raylib::math::{Vector2, Vector3};

/// Todo lo que un fragment shader sabe sobre un píxel cubierto por un triángulo.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code, reason = "API pública que todavía no usa la escena de prueba")]
pub struct Fragment {
    /// Píxel del framebuffer.
    pub x: usize,
    pub y: usize,
    pub depth: f32,
    /// Posición en el espacio del objeto, antes de transformarla. Los patrones
    /// procedurales se calculan aquí para que se muevan con el objeto.
    pub object_position: Vector3,
    pub world_position: Vector3,
    /// Normal interpolada y normalizada, en el espacio del objeto. No cambia
    /// cuando el objeto gira.
    pub object_normal: Vector3,
    /// Normal interpolada y normalizada, en coordenadas de mundo.
    pub normal: Vector3,
    pub uv: Vector2,
}
