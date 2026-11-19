mod config;
#[allow(dead_code, reason = "API pública que todavía no usa la escena de prueba")]
mod math;
mod mesh;
mod render;
mod scene;
mod shaders;
mod ship;
mod simulation;

use std::time::Instant;

use noise::OpenSimplex;
use raylib::prelude::*;

use config::{FAR_PLANE, FIELD_OF_VIEW, NEAR_PLANE, NOISE_SEED, RENDER_HEIGHT, RENDER_WIDTH, WINDOW_SCALE};
use math::Quat;
use render::{Background, Camera, FrameContext, Framebuffer, OrbitController, RenderStats};
use scene::{Detail, SolarSystem};
use scene::planets::PLANETS;
use ship::{ChaseCamera, PitchControl, Ship, ShipInput};
use simulation::Simulation;

/// Todo lo que se carga una sola vez al inicio.
struct World {
    scene: SolarSystem,
    background: Background,
    noise: OpenSimplex,
}

impl World {
    fn load() -> World {
        let noise = OpenSimplex::new(NOISE_SEED);
        World {
            scene: SolarSystem::load().unwrap_or_else(|err| panic!("{err}")),
            background: Background::new(&noise),
            noise,
        }
    }

    fn render(&self, framebuffer: &mut Framebuffer, camera: &Camera, time: f32, ship: &Ship, options: Options) -> RenderStats {
        let frame = FrameContext::new(
            camera,
            framebuffer.width,
            framebuffer.height,
            time,
            self.scene.light_position(),
            &self.noise,
        )
        .with_ship(ship.position)
        .with_culling(options.culling);
        let objects = self.scene.objects(time, ship, camera, options.detail);
        render::render_frame(framebuffer, &self.background, &objects, &frame)
    }
}

/// Optimizaciones del renderer que se pueden apagar para comparar cuánto
/// trabajo ahorran.
#[derive(Clone, Copy, PartialEq)]
struct Options {
    detail: Detail,
    culling: bool,
}

impl Options {
    const DEFAULT: Options = Options { detail: Detail::ByDistance, culling: true };
    /// Todo apagado: lo que costaría dibujar sin optimizaciones.
    const NONE: Options = Options { detail: Detail::Max, culling: false };

    /// Texto para el HUD, con la tecla que cambia cada opción.
    fn lines(&self) -> [String; 2] {
        let detail = match self.detail {
            Detail::ByDistance => "por distancia",
            Detail::Max => "siempre maximo",
        };
        let culling = if self.culling { "activo" } else { "apagado" };
        [format!("detalle de esferas: {detail} (L cambia)"), format!("descarte fuera de vista: {culling} (F cambia)")]
    }
}

/// Qué cámara se usa para mirar.
#[derive(Clone, Copy, PartialEq)]
enum View {
    /// Detrás de la nave. Es la del juego.
    Chase,
    /// Orbita alrededor de la nave para verla desde cualquier lado. Mientras
    /// está activa, la nave no recibe controles.
    Inspect,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let world = World::load();

    if args.iter().any(|arg| arg == "--benchmark") {
        benchmark(&world);
        return;
    }
    if let Some(index) = args.iter().position(|arg| arg == "--screenshot") {
        let prefix = args.get(index + 1).map_or("captura", String::as_str);
        screenshots(&world, prefix);
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

    let mut simulation = Simulation::new(&world.scene);
    let mut view = View::Chase;
    let mut inspect = OrbitController { distance: 10.0, ..OrbitController::new() };
    let mut pitch_control = PitchControl::Aircraft;
    let mut options = Options::DEFAULT;

    while !window.window_should_close() {
        let frame_time = window.get_frame_time();

        if window.is_key_pressed(KeyboardKey::KEY_SPACE) {
            simulation.paused = !simulation.paused;
        }
        if window.is_key_pressed(KeyboardKey::KEY_I) {
            pitch_control = pitch_control.toggled();
        }
        if window.is_key_pressed(KeyboardKey::KEY_L) {
            options.detail = if options.detail == Detail::ByDistance { Detail::Max } else { Detail::ByDistance };
        }
        if window.is_key_pressed(KeyboardKey::KEY_G) {
            simulation.gravity = !simulation.gravity;
        }
        if window.is_key_pressed(KeyboardKey::KEY_F) {
            options.culling = !options.culling;
        }
        if window.is_key_pressed(KeyboardKey::KEY_C) {
            view = if view == View::Chase { View::Inspect } else { View::Chase };
        }

        let input = match view {
            View::Chase => {
                if window.is_key_pressed(KeyboardKey::KEY_R) {
                    simulation.reset_ship(&world.scene);
                }
                ship::read_input(&window, pitch_control)
            }
            View::Inspect => {
                inspect.update(&window, frame_time);
                ShipInput::default()
            }
        };
        simulation.advance(&world.scene, &input, frame_time);

        let camera = match view {
            View::Chase => simulation.camera.camera(&simulation.ship, FIELD_OF_VIEW.to_radians(), NEAR_PLANE, FAR_PLANE),
            View::Inspect => {
                inspect.target = simulation.ship.position;
                inspect.camera(FIELD_OF_VIEW.to_radians(), NEAR_PLANE, FAR_PLANE)
            }
        };

        let start = Instant::now();
        let stats = world.render(&mut framebuffer, &camera, simulation.time, &simulation.ship, options);
        let render_ms = start.elapsed().as_secs_f32() * 1000.0;
        framebuffer.upload();

        let mut drawing = window.begin_drawing(&thread);
        framebuffer.draw(&mut drawing, WINDOW_SCALE as f32);
        draw_hud(&mut drawing, &world.scene, &stats, render_ms, &simulation, view, pitch_control, options);
        draw_exploration(&mut drawing, &world.scene, &simulation);
    }
}

/// Nombre y descripción del planeta cercano, planetas descubiertos y aviso de
/// temperatura junto al sol.
fn draw_exploration(drawing: &mut RaylibDrawHandle, scene: &SolarSystem, simulation: &Simulation) {
    let window_width = (RENDER_WIDTH * WINDOW_SCALE) as i32;
    let found = simulation.discovered.iter().filter(|found| **found).count();
    let counter = format!("planetas descubiertos: {found}/{}", PLANETS.len());
    let counter_width = drawing.measure_text(&counter, 20);
    drawing.draw_text(&counter, window_width - counter_width - 14, 10, 20, Color::GOLD);

    // El nombre aparece de a poco al acercarse.
    if let Some((index, near)) = scene.nearest_planet(simulation.time, simulation.ship.position) {
        let alpha = (near * 2.5).min(1.0);
        if alpha > 0.05 {
            let planet = &PLANETS[index];
            let fade = |color: Color| color.alpha(alpha);
            let name_width = drawing.measure_text(planet.name, 48);
            drawing.draw_text(planet.name, (window_width - name_width) / 2, 40, 48, fade(Color::RAYWHITE));
            let line_width = drawing.measure_text(planet.description, 22);
            drawing.draw_text(planet.description, (window_width - line_width) / 2, 96, 22, fade(Color::LIGHTGRAY));
        }
    }

    let heat = scene.ship_heat(simulation.time, simulation.ship.position);
    if heat > 0.3 {
        let warning = format!("TEMPERATURA DEL CASCO: {:.0}%", heat * 100.0);
        let width = drawing.measure_text(&warning, 26);
        // Parpadea más rápido cuanto más caliente.
        let blink = ((simulation.time * (3.0 + heat * 6.0)).sin() * 0.5 + 0.5) * 0.5 + 0.5;
        drawing.draw_text(&warning, (window_width - width) / 2, 132, 26, Color::ORANGERED.alpha(blink));
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_hud(
    drawing: &mut RaylibDrawHandle,
    scene: &SolarSystem,
    stats: &RenderStats,
    render_ms: f32,
    simulation: &Simulation,
    view: View,
    pitch_control: PitchControl,
    options: Options,
) {
    const SIZE: i32 = 20;
    let window_height = (RENDER_HEIGHT * WINDOW_SCALE) as i32;

    drawing.draw_fps(10, 10);
    drawing.draw_text(&format!("render: {render_ms:.1} ms"), 10, 34, SIZE, Color::RAYWHITE);
    for (row, line) in stats.lines().iter().enumerate() {
        drawing.draw_text(line, 10, 58 + row as i32 * 24, SIZE, Color::RAYWHITE);
    }
    drawing.draw_text(&format!("velocidad: {:.1}", simulation.ship.speed()), 10, 134, SIZE, Color::SKYBLUE);
    drawing.draw_text(&format!("cabeceo: {} (I cambia)", pitch_control.label()), 10, 158, SIZE, Color::SKYBLUE);
    for (row, line) in options.lines().iter().enumerate() {
        drawing.draw_text(line, 10, 182 + row as i32 * 24, SIZE, Color::LIME);
    }
    let gravity = if simulation.gravity {
        let colliders = scene.colliders(simulation.time);
        let position = simulation.ship.position;
        let pull = ship::gravity_at(position, &colliders).length();
        // El cuerpo que más jala, para saber hacia dónde se desvía la nave.
        let strongest = colliders
            .iter()
            .map(|body| (body.name, ship::gravity_at(position, std::slice::from_ref(body)).length()))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map_or("", |(name, _)| name);
        format!("gravedad: {pull:.2} u/s2, sobre todo {strongest} (G apaga)")
    } else {
        "gravedad: apagada (G cambia)".to_string()
    };
    drawing.draw_text(&gravity, 10, 230, SIZE, Color::SKYBLUE);
    if simulation.paused {
        drawing.draw_text("PAUSA", 10, 278, SIZE, Color::YELLOW);
    }

    let controls = match view {
        View::Chase => "W/S: acelerar/frenar   Flechas: dirigir (I invierte)   A/D: girar   Shift: turbo   G: gravedad   C: inspeccionar   R: reiniciar   Espacio: pausa",
        View::Inspect => "Flechas: orbitar la nave   Rueda: acercar   R: reiniciar vista   C: volver a la nave   Espacio: pausa",
    };
    drawing.draw_text(controls, 10, window_height - 30, SIZE, Color::LIGHTGRAY);
}

/// Poses fijas de la nave para medir y capturar siempre lo mismo.
fn fixed_views(world: &World) -> Vec<(&'static str, Ship)> {
    let turn = |angle: f32| Quat::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), angle);
    let nose_down = |angle: f32| Quat::from_axis_angle(Vector3::new(1.0, 0.0, 0.0), angle);
    // A 3 unidades de la superficie del primer planeta, al inicio de la
    // simulación, avanzando de lado con la punta un poco hacia abajo.
    let planet = &PLANETS[0];
    let surface = world.scene.planet_center(planet, 0.0) + Vector3::new(0.0, 0.0, planet.radius + 3.0);
    vec![
        ("inicio", world.scene.ship_start()),
        ("superficie", Ship::new(surface, turn(-std::f32::consts::FRAC_PI_2) * nose_down(0.35))),
        ("junto al sol", Ship::new(Vector3::new(0.0, 4.0, 22.0), turn(std::f32::consts::PI))),
    ]
}

/// Mide cuánto tarda un cuadro en cada pose fija, sin abrir ventana.
fn benchmark(world: &World) {
    const FRAMES: u32 = 60;
    let mut framebuffer = Framebuffer::new(RENDER_WIDTH, RENDER_HEIGHT);

    println!("{RENDER_WIDTH}x{RENDER_HEIGHT}, {FRAMES} cuadros por vista");
    for (name, ship) in fixed_views(world) {
        let camera = ChaseCamera::behind(&ship).camera(&ship, FIELD_OF_VIEW.to_radians(), NEAR_PLANE, FAR_PLANE);

        // Con y sin optimizaciones, para ver cuánto trabajo ahorran.
        for (label, options) in [("", Options::DEFAULT), (" (sin optimizar)", Options::NONE)] {
            let start = Instant::now();
            let mut stats = RenderStats::default();
            for _ in 0..FRAMES {
                stats = world.render(&mut framebuffer, &camera, 0.0, &ship, options);
            }
            let ms = start.elapsed().as_secs_f32() * 1000.0 / FRAMES as f32;

            println!("{name}{label}: {ms:.1} ms por cuadro");
            for line in stats.lines() {
                println!("    {line}");
            }
        }
    }
}

/// Guarda una captura por pose fija: `<prefijo>-<vista>.png`.
fn screenshots(world: &World, prefix: &str) {
    let mut framebuffer = Framebuffer::new(RENDER_WIDTH, RENDER_HEIGHT);
    for (name, ship) in fixed_views(world) {
        let camera = ChaseCamera::behind(&ship).camera(&ship, FIELD_OF_VIEW.to_radians(), NEAR_PLANE, FAR_PLANE);
        world.render(&mut framebuffer, &camera, 0.0, &ship, Options::DEFAULT);
        let path = format!("{prefix}-{}.png", name.replace(' ', "-"));
        framebuffer.export(&path);
        println!("Captura guardada en {path}");
    }

    // Todo el sistema visto desde arriba.
    let ship = world.scene.ship_start();
    let overview = OrbitController { target: Vector3::zero(), yaw: 0.3, pitch: 1.0, distance: 420.0 };
    world.render(&mut framebuffer, &overview.camera(FIELD_OF_VIEW.to_radians(), NEAR_PLANE, FAR_PLANE), 0.0, &ship, Options::DEFAULT);
    let path = format!("{prefix}-sistema.png");
    framebuffer.export(&path);
    println!("Captura guardada en {path}");

    // Cada planeta visto desde el lado del sol, un poco corrido para que se
    // vea la línea entre el día y la noche.
    for planet in PLANETS {
        let center = world.scene.planet_center(planet, 0.0);
        let towards_sun = -center;
        let view = OrbitController {
            target: center,
            yaw: towards_sun.x.atan2(towards_sun.z) + 0.6,
            pitch: 0.25,
            distance: planet.radius * 3.2,
        };
        let camera = view.camera(FIELD_OF_VIEW.to_radians(), NEAR_PLANE, FAR_PLANE);
        world.render(&mut framebuffer, &camera, 0.0, &ship, Options::DEFAULT);
        let name = planet.name.to_lowercase().replace(' ', "-");
        let path = format!("{prefix}-{name}.png");
        framebuffer.export(&path);
        println!("Captura guardada en {path}");

        // Lo mismo con la nave casi tocando la superficie, entre la cámara y el
        // planeta, para ver cómo reacciona. Se deja pasar un poco de tiempo para
        // que los efectos animados estén a media marcha.
        let towards_camera = (camera.position - center).normalize();
        let mut visitor = Ship::new(center + towards_camera * (planet.radius * 1.25), ship.orientation);
        visitor.throttle = 0.6;
        world.render(&mut framebuffer, &camera, 0.4, &visitor, Options::DEFAULT);
        let path = format!("{prefix}-{name}-cerca.png");
        framebuffer.export(&path);
        println!("Captura guardada en {path}");
    }

    // La nave de cerca, en tres cuartos y por detrás, con los motores a fondo.
    let mut ship = world.scene.ship_start();
    ship.throttle = 1.0;
    for (name, yaw) in [("nave-frente", 2.5), ("nave-cola", 0.5)] {
        let orbit = OrbitController { target: ship.position, yaw, pitch: 0.35, distance: 3.2 };
        let camera = orbit.camera(FIELD_OF_VIEW.to_radians(), NEAR_PLANE, FAR_PLANE);
        world.render(&mut framebuffer, &camera, 0.0, &ship, Options::DEFAULT);
        let path = format!("{prefix}-{name}.png");
        framebuffer.export(&path);
        println!("Captura guardada en {path}");
    }
}
