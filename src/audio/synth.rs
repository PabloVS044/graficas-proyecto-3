//! Sintetizador por software: todo el sonido del juego se calcula aquí, muestra
//! por muestra, sin archivos de audio. raylib solo lo reproduce.
//!
//! - `Engine` genera en tiempo real la capa continua: zumbido del espacio,
//!   motor, turbo y chasquidos de calor. Sus parámetros cambian cada cuadro y
//!   se suavizan muestra por muestra para que no haya saltos.
//! - Las funciones `chime`, `sweep`, `blip` y `alarm` generan efectos cortos una
//!   sola vez, al arrancar.
//! - `wav` empaqueta muestras en un archivo WAV en memoria para que raylib lo lea.

use std::f32::consts::TAU;

/// Muestras por segundo de todo el audio. Con 22 050 se oye bien hasta 11 kHz,
/// de sobra para estos sonidos, y cuesta la mitad de calcular.
pub const SAMPLE_RATE: u32 = 22_050;

/// Cuánto tardan los parámetros del motor en alcanzar el valor pedido, en
/// segundos. Evita chasquidos cuando el acelerador cambia de golpe.
const SMOOTHING: f32 = 0.08;

/// Lo que el motor necesita saber de la nave, cada cuadro.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EngineParams {
    /// Potencia de los motores, de 0 (apagados) a 1 (turbo).
    pub throttle: f32,
    /// 1 mientras el turbo está encendido, 0 si no.
    pub boost: f32,
    /// Calor del casco, de 0 a 1.
    pub heat: f32,
}

/// Generador de números aleatorios (xorshift) para el ruido.
struct Noise(u32);

impl Noise {
    /// Muestra de ruido blanco entre -1 y 1.
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

/// Filtro paso bajo de un polo: deja pasar lo grave y suaviza lo agudo. Más
/// alto `cutoff` (en Hz), más agudo se oye.
#[derive(Default)]
struct LowPass(f32);

impl LowPass {
    fn process(&mut self, input: f32, cutoff: f32) -> f32 {
        let alpha = 1.0 - (-TAU * cutoff / SAMPLE_RATE as f32).exp();
        self.0 += (input - self.0) * alpha;
        self.0
    }
}

/// La capa continua de sonido.
pub struct Engine {
    target: EngineParams,
    current: EngineParams,
    noise: Noise,
    engine_phase: f32,
    whine_phase: f32,
    pad_phases: [f32; 3],
    lfo_phase: f32,
    rumble: LowPass,
    hiss_low: LowPass,
    hiss_high: LowPass,
    crackle: f32,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            target: EngineParams::default(),
            current: EngineParams::default(),
            noise: Noise(0x1234_5678),
            engine_phase: 0.0,
            whine_phase: 0.0,
            pad_phases: [0.0; 3],
            lfo_phase: 0.0,
            rumble: LowPass::default(),
            hiss_low: LowPass::default(),
            hiss_high: LowPass::default(),
            crackle: 0.0,
        }
    }

    pub fn set(&mut self, params: EngineParams) {
        self.target = params;
    }

    /// Llena `out` con las siguientes muestras, entre -1 y 1.
    pub fn render(&mut self, out: &mut [f32]) {
        let dt = 1.0 / SAMPLE_RATE as f32;
        let follow = 1.0 - (-dt / SMOOTHING).exp();
        let advance = |phase: &mut f32, frequency: f32| {
            *phase = (*phase + frequency * dt).fract();
            *phase * TAU
        };

        for sample in out.iter_mut() {
            let c = &mut self.current;
            c.throttle += (self.target.throttle - c.throttle) * follow;
            c.boost += (self.target.boost - c.boost) * follow;
            c.heat += (self.target.heat - c.heat) * follow;
            let EngineParams { throttle, boost, heat } = *c;

            // Zumbido del espacio: tres tonos graves casi en quinta que suben y
            // bajan muy despacio. Siempre suena, bajito.
            let lfo = advance(&mut self.lfo_phase, 0.08).sin() * 0.5 + 0.5;
            let mut pad = 0.0;
            for (phase, frequency) in self.pad_phases.iter_mut().zip([55.0, 82.6, 110.4]) {
                pad += advance(phase, frequency).sin();
            }
            let pad = pad * (0.018 + lfo * 0.014);

            // Motor: un tono grave con armónicos que sube con el acelerador, más
            // un retumbe de ruido filtrado que se abre al acelerar.
            let angle = advance(&mut self.engine_phase, 38.0 + 52.0 * throttle);
            let tone = angle.sin() + (angle * 2.0).sin() * 0.5 + (angle * 3.0).sin() * 0.25;
            let rumble = self.rumble.process(self.noise.next(), 120.0 + 600.0 * throttle) * 3.0;
            let engine = tone * (0.04 + 0.1 * throttle) + rumble * (0.05 + 0.3 * throttle);

            // Turbo: un siseo (ruido sin lo más grave ni lo más agudo) y un silbido.
            let raw = self.noise.next();
            let hiss = self.hiss_high.process(raw, 3500.0) - self.hiss_low.process(raw, 600.0);
            let whine = advance(&mut self.whine_phase, 420.0 + 240.0 * boost).sin();
            let turbo = (hiss * 0.5 + whine * 0.05) * boost;

            // Calor: chasquidos sueltos, más seguidos cuanto más caliente.
            if self.noise.next() * 0.5 + 0.5 < heat * heat * 0.003 {
                self.crackle = 1.0;
            }
            self.crackle *= 0.993;
            let crackle = self.crackle * self.noise.next() * 0.35 * heat;

            // Saturación suave: nunca pasa de 1 aunque todo suene a la vez.
            *sample = (pad + engine + turbo + crackle).tanh();
        }
    }
}

/// Envolvente de un golpe: sube en `attack` segundos y cae exponencialmente.
fn envelope(t: f32, attack: f32, decay: f32) -> f32 {
    if t < attack { t / attack } else { (-(t - attack) / decay).exp() }
}

fn samples(seconds: f32) -> usize {
    (seconds * SAMPLE_RATE as f32) as usize
}

/// Acorde ascendente (do, mi, sol, do) para cuando se descubre un planeta.
pub fn chime() -> Vec<f32> {
    let notes = [523.25, 659.25, 783.99, 1046.5];
    let mut out = vec![0.0; samples(1.4)];
    for (index, frequency) in notes.iter().enumerate() {
        let start = samples(index as f32 * 0.11);
        for (i, sample) in out[start..].iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            let tone = (TAU * frequency * t).sin() + (TAU * frequency * 2.0 * t).sin() * 0.3;
            *sample += tone * envelope(t, 0.005, 0.35) * 0.18;
        }
    }
    out
}

/// Barrido de tono de `from` a `to` Hz en `seconds`: al entrar o salir del modo
/// orbital (subiendo o bajando).
pub fn sweep(from: f32, to: f32, seconds: f32) -> Vec<f32> {
    let mut phase: f32 = 0.0;
    (0..samples(seconds))
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let progress = t / seconds;
            // Sube en escala exponencial, como se oyen las notas.
            let frequency = from * (to / from).powf(progress);
            phase = (phase + frequency / SAMPLE_RATE as f32).fract();
            // Onda triangular: más suave que una cuadrada, con más cuerpo que un seno.
            let triangle = 1.0 - 4.0 * (phase - 0.5).abs();
            triangle * envelope(t, 0.01, seconds * 0.4) * 0.25
        })
        .collect()
}

/// Clic corto al cambiar una opción.
pub fn blip() -> Vec<f32> {
    (0..samples(0.07))
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            (TAU * 1250.0 * t).sin() * envelope(t, 0.002, 0.02) * 0.2
        })
        .collect()
}

/// Alarma de dos tonos para la temperatura del casco.
pub fn alarm() -> Vec<f32> {
    (0..samples(0.3))
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let frequency = if t < 0.15 { 880.0 } else { 660.0 };
            // Cuadrada suavizada: el seno saturado suena a alarma sin ser estridente.
            let square = ((TAU * frequency * t).sin() * 4.0).tanh();
            let gate = if t % 0.15 < 0.13 { 1.0 } else { 0.0 };
            square * gate * 0.12
        })
        .collect()
}

/// Archivo WAV (PCM de 16 bits, un canal) con las muestras, para que raylib
/// lo cargue desde memoria.
pub fn wav(samples: &[f32]) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes()); // tamaño del bloque de formato
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // un canal
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes por segundo
    bytes.extend_from_slice(&2u16.to_le_bytes()); // bytes por muestra
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits por muestra
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Volumen promedio (raíz del cuadrado medio) de un segundo del motor.
    fn loudness(params: EngineParams) -> f32 {
        let mut engine = Engine::new();
        engine.set(params);
        let mut out = vec![0.0; SAMPLE_RATE as usize];
        engine.render(&mut out);
        // Se mide la segunda mitad, cuando los parámetros ya llegaron.
        let tail = &out[out.len() / 2..];
        (tail.iter().map(|s| s * s).sum::<f32>() / tail.len() as f32).sqrt()
    }

    #[test]
    fn engine_gets_louder_with_throttle_and_boost() {
        let idle = loudness(EngineParams::default());
        let cruise = loudness(EngineParams { throttle: 0.6, ..EngineParams::default() });
        let boost = loudness(EngineParams { throttle: 1.0, boost: 1.0, heat: 0.0 });
        assert!(idle > 0.0, "el zumbido del espacio siempre suena");
        assert!(idle < cruise && cruise < boost, "{idle} {cruise} {boost}");
    }

    #[test]
    fn output_never_clips() {
        let mut engine = Engine::new();
        engine.set(EngineParams { throttle: 1.0, boost: 1.0, heat: 1.0 });
        let mut out = vec![0.0; SAMPLE_RATE as usize];
        engine.render(&mut out);
        assert!(out.iter().all(|s| s.abs() <= 1.0));
        for effect in [chime(), sweep(300.0, 900.0, 0.35), blip(), alarm()] {
            assert!(effect.iter().all(|s| s.abs() <= 1.0));
        }
    }

    #[test]
    fn wav_has_a_valid_header() {
        let bytes = wav(&[0.0, 0.5, -0.5]);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        assert_eq!(bytes.len(), 44 + 3 * 2);
        assert_eq!(i16::from_le_bytes([bytes[46], bytes[47]]), i16::MAX / 2);
    }

    #[test]
    fn sweep_starts_and_ends_quietly() {
        let sound = sweep(300.0, 900.0, 0.35);
        assert!(sound[0].abs() < 0.01);
        assert!(sound[sound.len() - 1].abs() < 0.05);
    }
}
