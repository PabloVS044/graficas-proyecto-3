//! Cámara: dónde está, hacia dónde mira y con qué lente.

use raylib::prelude::{KeyboardKey, RaylibHandle, Vector3};

use crate::math::Mat4;

/// Cámara en perspectiva. Quien la controle (por ahora `OrbitController`, luego
/// la cámara que sigue a la nave) solo tiene que actualizar estos campos.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub position: Vector3,
    pub target: Vector3,
    pub up: Vector3,
    /// Campo de visión vertical, en radianes.
    pub field_of_view: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn forward(&self) -> Vector3 {
        (self.target - self.position).normalize()
    }

    pub fn view(&self) -> Mat4 {
        Mat4::look_at(self.position, self.target, self.up)
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        Mat4::perspective(self.field_of_view, aspect, self.near, self.far)
    }

    pub fn rays(&self, width: usize, height: usize) -> CameraRays {
        let forward = self.forward();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward);
        let half_height = (self.field_of_view * 0.5).tan();
        let half_width = half_height * width as f32 / height as f32;

        CameraRays {
            forward,
            right: right * half_width,
            up: up * half_height,
            width: width as f32,
            height: height as f32,
        }
    }
}

/// Calcula hacia dónde mira cada píxel. Lo usa el fondo, que no es geometría.
#[derive(Clone, Copy, Debug)]
pub struct CameraRays {
    forward: Vector3,
    right: Vector3,
    up: Vector3,
    width: f32,
    height: f32,
}

impl CameraRays {
    /// Dirección normalizada del rayo que sale de la cámara por el píxel.
    pub fn direction(&self, x: usize, y: usize) -> Vector3 {
        let ndc_x = (x as f32 + 0.5) / self.width * 2.0 - 1.0;
        let ndc_y = 1.0 - (y as f32 + 0.5) / self.height * 2.0;
        (self.forward + self.right * ndc_x + self.up * ndc_y).normalize()
    }
}

/// Control provisional para revisar el renderer: orbita alrededor de un punto.
/// Se reemplaza por la cámara que sigue a la nave.
pub struct OrbitController {
    pub target: Vector3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl OrbitController {
    const START: OrbitController = OrbitController {
        target: Vector3 { x: 0.0, y: 0.0, z: 0.0 },
        yaw: 0.6,
        pitch: 0.35,
        distance: 45.0,
    };
    const MIN_DISTANCE: f32 = 1.0;
    const MAX_DISTANCE: f32 = 200.0;

    pub fn new() -> Self {
        Self::START
    }

    pub fn position(&self) -> Vector3 {
        self.target
            + Vector3::new(
                self.pitch.cos() * self.yaw.sin(),
                self.pitch.sin(),
                self.pitch.cos() * self.yaw.cos(),
            ) * self.distance
    }

    pub fn camera(&self, field_of_view: f32, near: f32, far: f32) -> Camera {
        Camera {
            position: self.position(),
            target: self.target,
            up: Vector3::new(0.0, 1.0, 0.0),
            field_of_view,
            near,
            far,
        }
    }

    /// Flechas: orbitar. Rueda: acercar (proporcional a la distancia). R: reiniciar.
    pub fn update(&mut self, window: &RaylibHandle, delta: f32) {
        const TURN_SPEED: f32 = 1.5;

        if window.is_key_down(KeyboardKey::KEY_LEFT) {
            self.yaw -= TURN_SPEED * delta;
        }
        if window.is_key_down(KeyboardKey::KEY_RIGHT) {
            self.yaw += TURN_SPEED * delta;
        }
        if window.is_key_down(KeyboardKey::KEY_UP) {
            self.pitch += TURN_SPEED * delta;
        }
        if window.is_key_down(KeyboardKey::KEY_DOWN) {
            self.pitch -= TURN_SPEED * delta;
        }
        self.pitch = self.pitch.clamp(-1.5, 1.5);

        let zoom = 1.0 - window.get_mouse_wheel_move() * 0.1;
        self.distance = (self.distance * zoom).clamp(Self::MIN_DISTANCE, Self::MAX_DISTANCE);

        if window.is_key_pressed(KeyboardKey::KEY_R) {
            *self = Self::START;
        }
    }
}
