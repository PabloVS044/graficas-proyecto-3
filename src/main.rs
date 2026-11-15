mod config;
#[allow(dead_code, reason = "API pública que todavía no usa la escena de prueba")]
mod math;
mod mesh;
mod render;
mod scene;
mod shaders;

use std::time::Instant;

use noise::OpenSimplex;
use raylib::prelude::*;

use config::{FAR_PLANE, FIELD_OF_VIEW, NEAR_PLANE, NOISE_SEED, RENDER_HEIGHT, RENDER_WIDTH, WINDOW_SCALE};
use render::{Background, Camera, FrameContext, Framebuffer, OrbitController, RenderStats};
use scene::DemoScene;

/// Un cuadro lento (por ejemplo al mover la ventana) no debe hacer saltar la simulación.
const MAX_FRAME_TIME: f32 = 0.1;

/// Todo lo que se carga una sola vez al inicio.
struct World {
    scene: DemoScene,
    background: Background,
    noise: OpenSimplex,
}

impl World {
    fn load() -> World {
        let noise = OpenSimplex::new(NOISE_SEED);
        World {
            scene: DemoScene::load().unwrap_or_else(|err| panic!("{err}")),
            background: Background::new(&noise),
            noise,
        }
    }

    fn render(&self, framebuffer: &mut Framebuffer, camera: &Camera, time: f32) -> RenderStats {
        let frame = FrameContext::new(
            camera,
            framebuffer.width,
            framebuffer.height,
            time,
            self.scene.light_position(),
            &self.noise,
        );
        let objects = self.scene.objects(time);
        render::render_frame(framebuffer, &self.background, &objects, &frame)
    }
}

fn camera_from(orbit: &OrbitController) -> Camera {
    orbit.camera(FIELD_OF_VIEW.to_radians(), NEAR_PLANE, FAR_PLANE)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let world = World::load();

    if args.iter().any(|arg| arg == "--benchmark") {
        benchmark(&world);
        return;
    }
    if let Some(index) = args.iter().position(|arg| arg == "--screenshot") {
        let path = args.get(index + 1).map_or("captura.png", String::as_str);
        screenshot(&world, path);
        return;
    }

    run_window(&world);
}

fn run_window(world: &World) {
    let (mut window, thread) = raylib::init()
        .size((RENDER_WIDTH * WINDOW_SCALE) as i32, (RENDER_HEIGHT * WINDOW_SCALE) as i32)
        .title("spaceTravel")
        .build();
    window.set_target_fps(60);

    let mut framebuffer = Framebuffer::new(RENDER_WIDTH, RENDER_HEIGHT);
    framebuffer.init_texture(&mut window, &thread);

    let mut orbit = OrbitController::new();
    let mut time = 0.0;
    let mut paused = false;

    while !window.window_should_close() {
        let delta = window.get_frame_time().min(MAX_FRAME_TIME);

        if window.is_key_pressed(KeyboardKey::KEY_SPACE) {
            paused = !paused;
        }
        if !paused {
            time += delta;
        }
        orbit.update(&window, delta);

        let start = Instant::now();
        let stats = world.render(&mut framebuffer, &camera_from(&orbit), time);
        let render_ms = start.elapsed().as_secs_f32() * 1000.0;
        framebuffer.upload();

        let mut drawing = window.begin_drawing(&thread);
        framebuffer.draw(&mut drawing, WINDOW_SCALE as f32);
        draw_hud(&mut drawing, &stats, render_ms, paused);
    }
}

fn draw_hud(drawing: &mut RaylibDrawHandle, stats: &RenderStats, render_ms: f32, paused: bool) {
    const SIZE: i32 = 20;
    let window_height = (RENDER_HEIGHT * WINDOW_SCALE) as i32;

    drawing.draw_fps(10, 10);
    drawing.draw_text(&format!("render: {render_ms:.1} ms"), 10, 34, SIZE, Color::RAYWHITE);
    for (row, line) in stats.lines().iter().enumerate() {
        drawing.draw_text(line, 10, 58 + row as i32 * 24, SIZE, Color::RAYWHITE);
    }
    if paused {
        drawing.draw_text("PAUSA", 10, 134, SIZE, Color::YELLOW);
    }
    drawing.draw_text(
        "Flechas: orbitar   Rueda: acercar   Espacio: pausa   R: reiniciar camara",
        10,
        window_height - 30,
        SIZE,
        Color::LIGHTGRAY,
    );
}

/// Mide cuánto tarda un cuadro desde varias distancias, sin abrir ventana.
/// La misma ruta en cada ejecución, para comparar entre versiones.
fn benchmark(world: &World) {
    const FRAMES: u32 = 60;
    let views = [("lejos", 80.0), ("medio", 40.0), ("cerca del sol", 12.0)];
    let mut framebuffer = Framebuffer::new(RENDER_WIDTH, RENDER_HEIGHT);

    println!("{RENDER_WIDTH}x{RENDER_HEIGHT}, {FRAMES} cuadros por vista");
    for (name, distance) in views {
        let orbit = OrbitController { distance, ..OrbitController::new() };
        let camera = camera_from(&orbit);

        let start = Instant::now();
        let mut stats = RenderStats::default();
        for frame in 0..FRAMES {
            stats = world.render(&mut framebuffer, &camera, frame as f32 / 60.0);
        }
        let ms = start.elapsed().as_secs_f32() * 1000.0 / FRAMES as f32;

        println!("{name}: {ms:.1} ms por cuadro");
        for line in stats.lines() {
            println!("    {line}");
        }
    }
}

/// Guarda un cuadro como PNG con la cámara inicial.
fn screenshot(world: &World, path: &str) {
    let mut framebuffer = Framebuffer::new(RENDER_WIDTH, RENDER_HEIGHT);
    world.render(&mut framebuffer, &camera_from(&OrbitController::new()), 2.0);
    framebuffer.export(path);
    println!("Captura guardada en {path}");
}
