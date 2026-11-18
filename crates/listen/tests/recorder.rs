//! A run's recording and cut log survive rcnc dying mid-run (the op1-top crash, 2026-11-18, lost both: they
//! were only written when the program stopped).

use cnc_listen::{read_wav, CutLog, WavRecorder};

#[test]
fn a_recording_cut_off_mid_run_is_a_wav_up_to_its_last_flush() {
    let dir = std::env::temp_dir().join(format!("rcnc-rec-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("op1-top.nc.wav");
    let rate = 24_000;
    let tone: Vec<f32> = (0..rate).map(|i| (i as f32 * 0.05).sin() * 0.5).collect();
    let mut r = WavRecorder::create(&path, rate, 3600.0).unwrap();
    r.push(&tone).unwrap();
    r.push(&tone).unwrap();
    r.flush().unwrap();
    r.push(&tone).unwrap();
    // the process dies here: nothing finalizes the file
    std::mem::forget(r);
    let (x, got_rate) = read_wav(&path).unwrap();
    assert_eq!(got_rate, rate);
    assert!(x.len() >= 2 * rate as usize, "{} samples", x.len());
    assert!((x[100] - tone[100]).abs() < 1e-3);
    // and the cap holds
    let capped = dir.join("capped.wav");
    let mut r = WavRecorder::create(&capped, rate, 1.5).unwrap();
    for _ in 0..3 {
        r.push(&tone).unwrap();
    }
    assert_eq!(r.samples(), rate as usize * 3 / 2);
    r.finish().unwrap();
    assert_eq!(read_wav(&capped).unwrap().0.len(), rate as usize * 3 / 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn saving_the_cut_log_replaces_it_whole() {
    let dir = std::env::temp_dir().join(format!("rcnc-log-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("op1-top.nc.cutlog.json");
    let log = CutLog::new("op1-top.nc", "test");
    log.save(&path).unwrap();
    log.save(&path).unwrap();
    assert!(CutLog::load(&path).is_ok());
    assert!(!dir.join("op1-top.nc.cutlog.json.part").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
