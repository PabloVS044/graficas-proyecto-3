//! Sprites: puntos brillantes en el mundo, como las partículas del escape.
//!
//! No son triángulos. Cada sprite se proyecta como un punto y se pinta como un
//! disco de luz que se desvanece hacia el borde, del tamaño que tendría a esa
//! distancia. Van después de los objetos: se prueba la profundidad para que una
//! nave o un planeta delante los tape, pero no la escriben, y su color se suma
//! al que ya hay (mezcla aditiva). Así muchas partículas juntas brillan más y
//! el orden en que se dibujan no importa.

use raylib::math::{Vector3, Vector4};

use super::{FrameContext, Framebuffer};

#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub position: Vector3,
    /// Radio en unidades del mundo.
    pub radius: f32,
    /// Luz que suma en el centro del disco. Puede pasar de 1.
    pub color: Vector3,
}

/// Radio máximo en pantalla: una partícula pegada a la cámara no debe cubrir
/// media pantalla ni costar demasiado.
const MAX_RADIUS_PIXELS: f32 = 40.0;
/// Radio mínimo: una partícula lejana sigue ocupando un píxel.
const MIN_RADIUS_PIXELS: f32 = 0.8;

/// Dibuja los sprites y devuelve cuántos llegaron a la pantalla.
pub fn draw_sprites(framebuffer: &mut Framebuffer, sprites: &[Sprite], frame: &FrameContext) -> usize {
    let (width, height) = (framebuffer.width, framebuffer.height);
    // Cuántos píxeles mide una unidad del mundo a distancia 1: la escala vertical
    // de la proyección por media altura de la pantalla.
    let pixels_per_unit = frame.projection.at(1, 1) * height as f32 * 0.5;
    let mut drawn = 0;

    for sprite in sprites {
        let p = sprite.position;
        let clip = frame.view_projection.transform(Vector4::new(p.x, p.y, p.z, 1.0));
        // Detrás de la cámara o pegado a ella.
        if clip.w <= frame.near {
            continue;
        }

        let inverse_w = 1.0 / clip.w;
        let center_x = (clip.x * inverse_w + 1.0) * 0.5 * width as f32;
        let center_y = (1.0 - clip.y * inverse_w) * 0.5 * height as f32;
        let radius = (sprite.radius * pixels_per_unit * inverse_w).clamp(MIN_RADIUS_PIXELS, MAX_RADIUS_PIXELS);

        // Recuadro del disco, recortado a la pantalla.
        let left = (center_x - radius).floor().max(0.0) as usize;
        let top = (center_y - radius).floor().max(0.0) as usize;
        let right = ((center_x + radius).ceil() as isize).min(width as isize - 1);
        let bottom = ((center_y + radius).ceil() as isize).min(height as isize - 1);
        if right < left as isize || bottom < top as isize {
            continue;
        }

        let mut visible = false;
        for y in top..=bottom as usize {
            for x in left..=right as usize {
                let dx = (x as f32 + 0.5 - center_x) / radius;
                let dy = (y as f32 + 0.5 - center_y) / radius;
                let distance = dx * dx + dy * dy;
                if distance >= 1.0 {
                    continue;
                }
                let index = y * width + x;
                // Profundidad con `1/w`: más grande = más cerca. Si lo que ya está
                // dibujado está más cerca, tapa a la partícula.
                if framebuffer.depths[index] > inverse_w {
                    continue;
                }
                let falloff = (1.0 - distance) * (1.0 - distance);
                framebuffer.colors[index] += sprite.color * falloff;
                visible = true;
            }
        }
        drawn += visible as usize;
    }

    drawn
}

#[cfg(test)]
mod tests {
    use noise::OpenSimplex;

    use super::*;
    use crate::render::Camera;

    fn setup() -> (Framebuffer, FrameContext) {
        let camera = Camera {
            position: Vector3::zero(),
            target: Vector3::new(0.0, 0.0, -1.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            field_of_view: 60f32.to_radians(),
            near: 0.1,
            far: 100.0,
        };
        let frame = FrameContext::new(&camera, 64, 64, 0.0, Vector3::zero(), &OpenSimplex::new(1));
        (Framebuffer::new(64, 64), frame)
    }

    fn sprite(z: f32) -> Sprite {
        Sprite { position: Vector3::new(0.0, 0.0, z), radius: 0.5, color: Vector3::one() }
    }

    #[test]
    fn sprite_in_front_adds_light_at_the_center() {
        let (mut framebuffer, frame) = setup();
        assert_eq!(draw_sprites(&mut framebuffer, &[sprite(-5.0)], &frame), 1);
        let center = framebuffer.colors[32 * 64 + 32];
        assert!(center.x > 0.8);
        assert_eq!(framebuffer.colors[0], Vector3::zero(), "la esquina queda igual");
    }

    #[test]
    fn sprites_behind_the_camera_or_hidden_are_skipped() {
        let (mut framebuffer, frame) = setup();
        assert_eq!(draw_sprites(&mut framebuffer, &[sprite(5.0)], &frame), 0, "detrás de la cámara");

        // Algo más cerca que la partícula ya ocupa toda la pantalla.
        framebuffer.depths.fill(1.0);
        assert_eq!(draw_sprites(&mut framebuffer, &[sprite(-5.0)], &frame), 0, "tapada");
    }

    #[test]
    fn overlapping_sprites_add_up() {
        let (mut one, frame) = setup();
        draw_sprites(&mut one, &[sprite(-5.0)], &frame);
        let (mut two, _) = setup();
        draw_sprites(&mut two, &[sprite(-5.0), sprite(-5.0)], &frame);
        let center = 32 * 64 + 32;
        assert!((two.colors[center].x - one.colors[center].x * 2.0).abs() < 1e-5);
    }
}
