//! A tap test: tap the tool in the spindle with something hard and listen to it ring. The ring's
//! frequency is the tool's first bending mode at that stick-out and how fast it dies away its damping:
//! rCAD's chatter model takes both (the tool table's mode_hz and mode_zeta).

use rustfft::{num_complex::Complex, FftPlanner};

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Ring {
    /// Natural frequency, Hz, and damping ratio.
    pub hz: f64,
    pub zeta: f64,
    /// How far the ring stood above the noise before the tap, dB (a weak tap gives a poor estimate).
    pub snr_db: f64,
}

/// Find the tap (the loudest moment) and fit the ring after it: the strongest peak between `lo` and `hi`
/// Hz in the spectrum of the ring, and its half-power width (a ring that dies away at ζ·ω has a peak
/// 2ζ·f wide: a tool rings for only a few milliseconds, too short to follow its decay directly).
pub fn analyse(x: &[f32], rate: u32, lo: f64, hi: f64) -> Result<Ring, String> {
    let r = rate as f64;
    let onset = x.iter().enumerate().max_by(|a, b| a.1.abs().total_cmp(&b.1.abs())).map(|(i, _)| i).ok_or("no sound")?;
    // a little after the hit itself (it rings everything), 40 ms of ring, zero-padded for a fine spectrum
    let start = onset + (0.0005 * r) as usize;
    let len = (0.04 * r) as usize;
    if start + len > x.len() {
        return Err("the recording stops too soon after the tap".into());
    }
    let n = 1 << 16;
    let fft = FftPlanner::<f32>::new().plan_fft_forward(n);
    let spec = |at: usize| -> Vec<f64> {
        let mut c: Vec<Complex<f32>> = (0..n).map(|i| Complex::new(if i < len { x[at + i] } else { 0.0 }, 0.0)).collect();
        fft.process(&mut c);
        c[..n / 2].iter().map(|v| (v.norm() as f64).powi(2)).collect()
    };
    let bin = r / n as f64;
    let p = spec(start);
    let (a, b) = (((lo / bin) as usize).max(2), ((hi / bin) as usize).min(n / 2 - 2));
    let k = (a..=b).max_by(|i, j| p[*i].total_cmp(&p[*j])).ok_or("no ring in range")?;
    let half = p[k] / 2.0;
    // where the power falls to half on each side, interpolated
    let mut l = k;
    while l > a && p[l] > half {
        l -= 1;
    }
    let mut h = k;
    while h < b && p[h] > half {
        h += 1;
    }
    let edge = |i: usize, j: usize| i as f64 + (half - p[i]) / (p[j] - p[i]).max(1e-30) * (j as f64 - i as f64);
    let (fl, fh) = (edge(l, l + 1) * bin, edge(h, h - 1) * bin);
    let hz = k as f64 * bin;
    let zeta = ((fh - fl) / (2.0 * hz)).max(0.0);
    // the same stretch before the tap: the noise
    let snr_db = if onset > len + 10 { 10.0 * (p[k] / spec(onset - len - 10)[k].max(1e-30)).log10() } else { f64::INFINITY };
    if snr_db < 10.0 {
        return Err(format!("the ring is only {snr_db:.0} dB above the noise: tap harder, closer to the tip"));
    }
    Ok(Ring { hz, zeta, snr_db })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_ring_and_its_damping() {
        let rate = 48_000u32;
        let (hz, zeta) = (4200.0, 0.02);
        let w = 2.0 * std::f64::consts::PI * hz;
        let mut x = vec![0.0f32; 24_000];
        let mut seed = 7u64;
        for s in x.iter_mut() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *s = ((seed >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.001;
        }
        let t0 = 6000;
        for i in t0..x.len() {
            let t = (i - t0) as f64 / rate as f64;
            x[i] += (0.8 * (-zeta * w * t).exp() * (w * (1.0 - zeta * zeta).sqrt() * t).sin()) as f32;
        }
        x[t0] = 0.95;
        let ring = analyse(&x, rate, 500.0, 15_000.0).unwrap();
        assert!((ring.hz - hz).abs() < 15.0, "{ring:?}");
        assert!((ring.zeta - zeta).abs() < 0.006, "{ring:?}");
    }
}
