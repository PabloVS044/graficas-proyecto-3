//! Menú de inicio: elegir la nave antes de despegar.
//!
//! La vista previa no es una imagen: es la nave girando, dibujada con el mismo
//! renderer que el juego, con su shader y los motores encendidos.

use raylib::prelude::*;

use crate::config::{RENDER_HEIGHT, RENDER_WIDTH, WINDOW_SCALE};
use crate::math::Quat;
use crate::render::{Camera, OrbitController};
use crate::scene::{SceneObject, SolarSystem};
use crate::ship::{SHIPS, Ship};

/// Radianes por segundo que gira la nave en la vista previa.
const SPIN_SPEED: f32 = 0.6;
/// Desde dónde la ve la cámara.
const CAMERA_DISTANCE: f32 = 3.4;
const CAMERA_PITCH: f32 = 0.32;
const CAMERA_YAW: f32 = 0.9;
/// La luz de la vista previa: arriba, a un lado y un poco atrás de la cámara.
const LIGHT_OFFSET: Vector3 = Vector3 { x: 40.0, y: 60.0, z: 80.0 };

/// Qué pidió quien juega en este cuadro.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuAction {
    None,
    /// Cambió la nave seleccionada.
    Changed,
    /// Eligió la nave y quiere despegar.
    Launch,
}

pub struct ShipMenu {
    /// Índice en `SHIPS`.
    pub selected: usize,
}

impl ShipMenu {
    pub fn new(selected: usize) -> ShipMenu {
        ShipMenu { selected }
    }

    pub fn next(&mut self) {
        self.selected = (self.selected + 1) % SHIPS.len();
    }

    pub fn previous(&mut self) {
        self.selected = (self.selected + SHIPS.len() - 1) % SHIPS.len();
    }

    /// Flechas izquierda y derecha cambian de nave; Enter despega.
    pub fn update(&mut self, window: &RaylibHandle) -> MenuAction {
        if window.is_key_pressed(KeyboardKey::KEY_RIGHT) {
            self.next();
            return MenuAction::Changed;
        }
        if window.is_key_pressed(KeyboardKey::KEY_LEFT) {
            self.previous();
            return MenuAction::Changed;
        }
        if window.is_key_pressed(KeyboardKey::KEY_ENTER) || window.is_key_pressed(KeyboardKey::KEY_KP_ENTER) {
            return MenuAction::Launch;
        }
        MenuAction::None
    }

    /// La nave seleccionada girando, la cámara que la mira y dónde está la luz.
    /// `time` son segundos reales desde que se abrió el menú.
    pub fn preview(&self, scene: &SolarSystem, time: f32) -> (Camera, Vec<SceneObject>, Vector3) {
        let center = Vector3::zero();
        let spin = Quat::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), time * SPIN_SPEED);
        let mut ship = Ship::new(center, spin);
        // Motores a media marcha, con un pulso lento para que se vean vivos.
        ship.throttle = 0.45 + 0.2 * (time * 1.7).sin();

        let orbit = OrbitController { target: center, yaw: CAMERA_YAW, pitch: CAMERA_PITCH, distance: CAMERA_DISTANCE };
        let camera = orbit.camera(crate::config::FIELD_OF_VIEW.to_radians(), crate::config::NEAR_PLANE, crate::config::FAR_PLANE);
        (camera, vec![scene.ship_object(self.selected, &ship, 0.0)], center + LIGHT_OFFSET)
    }

    /// Título, datos de la nave y controles, encima de la vista previa.
    pub fn draw(&self, drawing: &mut RaylibDrawHandle, scene: &SolarSystem) {
        let width = (RENDER_WIDTH * WINDOW_SCALE) as i32;
        let height = (RENDER_HEIGHT * WINDOW_SCALE) as i32;
        let centered = |drawing: &mut RaylibDrawHandle, text: &str, y: i32, size: i32, color: Color| {
            let text_width = drawing.measure_text(text, size);
            drawing.draw_text(text, (width - text_width) / 2, y, size, color);
        };

        centered(drawing, "spaceTravel", 40, 64, Color::RAYWHITE);
        centered(drawing, "Elige tu nave", 116, 28, Color::LIGHTGRAY);

        let spec = &SHIPS[self.selected];
        let name = format!("<   {}   >", spec.name);
        centered(drawing, &name, height - 270, 56, Color::GOLD);
        centered(drawing, &format!("Modelada por {}", spec.author), height - 200, 26, Color::RAYWHITE);
        centered(drawing, spec.description, height - 164, 22, Color::LIGHTGRAY);
        let details = format!("{} triangulos   -   nave {} de {}", scene.ship_triangles(self.selected), self.selected + 1, SHIPS.len());
        centered(drawing, &details, height - 128, 20, Color::GRAY);

        centered(drawing, "Flechas izq/der: cambiar de nave     Enter: despegar     M: sonido", height - 50, 22, Color::SKYBLUE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_wrap_around_the_list() {
        let mut menu = ShipMenu::new(0);
        menu.previous();
        assert_eq!(menu.selected, SHIPS.len() - 1);
        menu.next();
        assert_eq!(menu.selected, 0);
        for _ in 0..SHIPS.len() {
            menu.next();
        }
        assert_eq!(menu.selected, 0);
    }

    /// La vista previa dibuja solo la nave elegida, frente a la cámara.
    #[test]
    fn preview_shows_the_selected_ship() {
        let scene = SolarSystem::load().unwrap();
        for (index, spec) in SHIPS.iter().enumerate() {
            let menu = ShipMenu::new(index);
            let (camera, objects, _) = menu.preview(&scene, 1.0);
            assert_eq!(objects.len(), 1);
            assert_eq!(objects[0].name, spec.name);
            let to_ship = (objects[0].model.origin() - camera.position).normalize();
            assert!(to_ship.dot(camera.forward()) > 0.9);
        }
    }
}
