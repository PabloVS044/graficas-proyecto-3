//! Parámetros globales del programa. Los parámetros de cada planeta, shader o
//! sistema viven junto a su código; aquí solo va lo que comparte todo el motor.

/// Resolución a la que se renderiza. Es menor que la ventana porque el costo
/// del renderer crece con la cantidad de píxeles.
pub const RENDER_WIDTH: usize = 800;
pub const RENDER_HEIGHT: usize = 450;

/// Cuántas veces se amplía el framebuffer al mostrarlo en la ventana.
pub const WINDOW_SCALE: usize = 2;

/// Campo de visión vertical, en grados.
pub const FIELD_OF_VIEW: f32 = 60.0;

/// Distancia al plano cercano y al lejano de la proyección.
pub const NEAR_PLANE: f32 = 0.1;
pub const FAR_PLANE: f32 = 1000.0;

/// Semilla de todo el ruido procedural. Misma semilla, mismo sistema solar.
pub const NOISE_SEED: u32 = 7;

pub const SHIP_MODEL_PATH: &str = "assets/nave_graficas.obj";
/// Esfera para las pruebas del cargador OBJ. La escena usa la icosfera.
#[cfg(test)]
pub const SPHERE_MODEL_PATH: &str = "assets/sphere.obj";
