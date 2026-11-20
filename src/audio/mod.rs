//! Sonido del juego. Todo se sintetiza en `synth`; aquí solo se le entrega a
//! raylib para que lo reproduzca.
//!
//! La capa continua (motor, turbo, espacio, calor) va por un stream: raylib
//! avisa cuando terminó de tocar un bloque de muestras y se le da el siguiente,
//! recién calculado con los parámetros de la nave. Los efectos cortos se
//! generan una vez al arrancar y se cargan como sonidos normales.

mod synth;

use raylib::core::audio::{AudioStream, RaylibAudio, Sound};

pub use synth::EngineParams;
use synth::{Engine, SAMPLE_RATE, alarm, blip, chime, sweep, wav};

/// Muestras por bloque del stream: unos 140 ms. Más grande aguanta cuadros más
/// lentos sin cortes, pero el sonido responde con más retraso al acelerador.
const BLOCK: usize = 3072;

/// Efectos cortos que dispara el juego.
#[derive(Clone, Copy, Debug)]
pub enum Cue {
    /// Se descubrió un planeta.
    Discovery,
    /// Se entró en modo orbital.
    OrbitOn,
    /// Se volvió al vuelo libre.
    OrbitOff,
    /// Se cambió una opción con el teclado.
    Toggle,
}

pub struct Soundscape<'a> {
    audio: &'a RaylibAudio,
    stream: AudioStream<'a>,
    engine: Engine,
    block: Vec<f32>,
    discovery: Sound<'a>,
    orbit_on: Sound<'a>,
    orbit_off: Sound<'a>,
    toggle: Sound<'a>,
    alarm: Sound<'a>,
    /// Segundos para la siguiente alarma de temperatura.
    alarm_countdown: f32,
    muted: bool,
}

impl<'a> Soundscape<'a> {
    pub fn load(audio: &'a RaylibAudio) -> Result<Soundscape<'a>, String> {
        let sound = |samples: Vec<f32>| -> Result<Sound<'a>, String> {
            let wave = audio.new_wave_from_memory(".wav", &wav(&samples)).map_err(|err| format!("{err:?}"))?;
            audio.new_sound_from_wave(&wave).map_err(|err| format!("{err:?}"))
        };

        audio.set_audio_stream_buffer_size_default(BLOCK as i32);
        // Muestras `f32` (32 bits), un canal.
        let stream = audio.new_audio_stream(SAMPLE_RATE, 32, 1);
        stream.play();

        Ok(Soundscape {
            audio,
            stream,
            engine: Engine::new(),
            block: vec![0.0; BLOCK],
            discovery: sound(chime())?,
            orbit_on: sound(sweep(300.0, 900.0, 0.35))?,
            orbit_off: sound(sweep(900.0, 300.0, 0.35))?,
            toggle: sound(blip())?,
            alarm: sound(alarm())?,
            alarm_countdown: 0.0,
            muted: false,
        })
    }

    /// Se llama cada cuadro: actualiza el motor con el estado de la nave, le da
    /// al stream los bloques que pida y hace sonar la alarma si hace calor.
    pub fn update(&mut self, params: EngineParams, frame_time: f32) {
        self.engine.set(params);
        // Como mucho dos bloques: el stream tiene dos y no puede pedir más.
        for _ in 0..2 {
            if !self.stream.is_processed() {
                break;
            }
            self.engine.render(&mut self.block);
            // Solo falla si el tamaño de muestra no coincide, y aquí siempre es `f32`.
            let _ = self.stream.update(&self.block);
        }

        // La alarma se repite más seguido cuanto más caliente está el casco.
        if params.heat > 0.3 {
            self.alarm_countdown -= frame_time;
            if self.alarm_countdown <= 0.0 {
                self.alarm.play();
                self.alarm_countdown = 1.3 - params.heat * 0.7;
            }
        } else {
            self.alarm_countdown = 0.0;
        }
    }

    pub fn play(&self, cue: Cue) {
        match cue {
            Cue::Discovery => self.discovery.play(),
            Cue::OrbitOn => self.orbit_on.play(),
            Cue::OrbitOff => self.orbit_off.play(),
            Cue::Toggle => self.toggle.play(),
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.audio.set_master_volume(if self.muted { 0.0 } else { 1.0 });
    }

    pub fn muted(&self) -> bool {
        self.muted
    }
}
