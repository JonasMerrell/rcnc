//! Live capture from a sound card (the `audio` feature): cpal runs its own thread and drops mono samples
//! into a buffer the app empties a few times a second. Nothing here touches the machine.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct Capture {
    _stream: cpal::Stream,
    buf: Arc<Mutex<Vec<f32>>>,
    rate: u32,
    name: String,
}

/// The names of the input devices.
pub fn inputs() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices().map(|d| d.filter_map(|d| d.name().ok()).collect()).unwrap_or_default()
}

impl Capture {
    /// Start listening on the input called `name` (None: the default input).
    pub fn start(name: Option<&str>) -> Result<Capture, String> {
        let host = cpal::default_host();
        let device = match name {
            Some(n) => host.input_devices().map_err(|e| e.to_string())?.find(|d| d.name().is_ok_and(|x| x == n)).ok_or(format!("no input called {n}"))?,
            None => host.default_input_device().ok_or("no audio input device")?,
        };
        let name = device.name().unwrap_or_else(|_| "input".into());
        let config = device.default_input_config().map_err(|e| format!("{name}: {e}"))?;
        let rate = config.sample_rate().0;
        let channels = config.channels().max(1) as usize;
        let buf = Arc::new(Mutex::new(Vec::<f32>::new()));
        let sink = buf.clone();
        // keep at most 10 s if nobody reads it
        let cap = rate as usize * 10;
        let push = move |mono: &mut dyn Iterator<Item = f32>| {
            if let Ok(mut b) = sink.lock() {
                b.extend(mono);
                if b.len() > cap {
                    let extra = b.len() - cap;
                    b.drain(..extra);
                }
            }
        };
        let err = |e| eprintln!("audio input: {e}");
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.config(),
                move |d: &[f32], _: &_| {
                    let push = push.clone();
                    push(&mut d.chunks(channels).map(|c| c[0]))
                },
                err,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.config(),
                move |d: &[i16], _: &_| {
                    let push = push.clone();
                    push(&mut d.chunks(channels).map(|c| c[0] as f32 / 32768.0))
                },
                err,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.config(),
                move |d: &[u16], _: &_| {
                    let push = push.clone();
                    push(&mut d.chunks(channels).map(|c| (c[0] as f32 - 32768.0) / 32768.0))
                },
                err,
                None,
            ),
            f => return Err(format!("{name}: sample format {f:?} not supported")),
        }
        .map_err(|e| format!("{name}: {e}"))?;
        stream.play().map_err(|e| format!("{name}: {e}"))?;
        Ok(Capture { _stream: stream, buf, rate, name })
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn take(&mut self) -> Vec<f32> {
        self.buf.lock().map(|mut b| std::mem::take(&mut *b)).unwrap_or_default()
    }
}
