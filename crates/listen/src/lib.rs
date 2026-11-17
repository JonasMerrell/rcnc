//! Listening to the cut. A microphone (or a contact pickup on the spindle) hears the spindle and the
//! teeth hitting the work; from a rolling spectrum this crate finds:
//!
//! - the spindle's real speed, from the tooth-passing frequency (rpm × teeth / 60) or, off the cut, the
//!   rotation frequency: so bogging shows as speed droop under load;
//! - chatter: a strong peak that isn't a harmonic of the spindle's rotation;
//! - how loud the cut is.
//!
//! It logs them per program line (`CutLog`, written next to the program as `<program>.cutlog.json`) for
//! rCAD to calibrate its cutting model from, and measures a tool's ring after a tap (`tap`). Live capture
//! from a sound card needs the `audio` feature; a WAV file can stand in (tests, the simulator).

pub mod analysis;
#[cfg(feature = "audio")]
pub mod capture;
pub mod cutlog;
pub mod guard;
pub mod limiter;
pub mod tap;

pub use analysis::{Analyzer, Context, Frame};
pub use cutlog::{CutLog, Moment};
pub use guard::{Guard, GuardSettings};
pub use limiter::{Limiter, LimiterSettings};

/// Where samples come from: a sound card or a WAV file played back in real time.
pub enum Source {
    #[cfg(feature = "audio")]
    Live(capture::Capture),
    Wav { samples: Vec<f32>, rate: u32, at: usize, start: std::time::Instant },
}

impl Source {
    /// A mono WAV file (the first channel of a multichannel one), played back from now.
    pub fn wav(path: &std::path::Path) -> Result<Source, String> {
        let (samples, rate) = read_wav(path)?;
        Ok(Source::Wav { samples, rate, at: 0, start: std::time::Instant::now() })
    }

    pub fn rate(&self) -> u32 {
        match self {
            #[cfg(feature = "audio")]
            Source::Live(c) => c.rate(),
            Source::Wav { rate, .. } => *rate,
        }
    }

    /// The samples that have come in since the last call.
    pub fn take(&mut self) -> Vec<f32> {
        match self {
            #[cfg(feature = "audio")]
            Source::Live(c) => c.take(),
            Source::Wav { samples, rate, at, start } => {
                let due = ((start.elapsed().as_secs_f64() * *rate as f64) as usize).min(samples.len());
                let out = samples[(*at).min(due)..due].to_vec();
                *at = due;
                out
            }
        }
    }

    pub fn name(&self) -> String {
        match self {
            #[cfg(feature = "audio")]
            Source::Live(c) => c.name().to_string(),
            Source::Wav { .. } => "WAV file".into(),
        }
    }
}

/// The input devices there are (empty without the `audio` feature or a sound card).
pub fn inputs() -> Vec<String> {
    #[cfg(feature = "audio")]
    {
        capture::inputs()
    }
    #[cfg(not(feature = "audio"))]
    {
        vec![]
    }
}

/// Open a sound card input by name (None: the default one).
pub fn open(name: Option<&str>) -> Result<Source, String> {
    #[cfg(feature = "audio")]
    {
        capture::Capture::start(name).map(Source::Live)
    }
    #[cfg(not(feature = "audio"))]
    {
        let _ = name;
        Err("this build of rcnc has no audio input (built without the `audio` feature)".into())
    }
}

/// Read a WAV file as mono f32 samples in −1..1, and its sample rate.
pub fn read_wav(path: &std::path::Path) -> Result<(Vec<f32>, u32), String> {
    let mut r = hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let spec = r.spec();
    let ch = spec.channels.max(1) as usize;
    let all: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample.saturating_sub(1))) as f32;
            r.samples::<i32>().map(|s| s.unwrap_or(0) as f32 / scale).collect()
        }
    };
    Ok((all.iter().step_by(ch).copied().collect(), spec.sample_rate))
}

/// Write mono f32 samples as a 16-bit WAV (tests, and saving a tap test's recording).
pub fn write_wav(path: &std::path::Path, samples: &[f32], rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec { channels: 1, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for s in samples {
        w.write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}
