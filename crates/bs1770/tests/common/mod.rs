//! Signal synthesis for the EBU minimum-requirements tests. Everything here follows the wording of
//! Tech 3341 Table 1 and Tech 3342 Table 1: levels are per-channel PEAK levels in dBFS, tones are
//! 1 kHz unless the case says otherwise, stereo tones are in phase.
#![allow(dead_code)]

use bs1770::Meter;
use std::f64::consts::PI;

pub const RATE: f64 = 48_000.0;

pub fn amp(dbfs: f64) -> f64 {
    10f64.powf(dbfs / 20.0)
}

/// Append `seconds` of a sine at `hz` with per-channel peak `dbfs` (one entry per channel) and a
/// starting phase in degrees, continuing the phase from `phase_acc` so segments join cleanly.
pub fn tone(
    meter: &mut Meter,
    seconds: f64,
    hz: f64,
    dbfs_per_channel: &[f64],
    phase_deg: f64,
    phase_acc: &mut f64,
) {
    let n = (seconds * RATE).round() as usize;
    let amps: Vec<f64> = dbfs_per_channel.iter().map(|&d| amp(d)).collect();
    let mut frame = vec![0.0; amps.len()];
    let w = 2.0 * PI * hz / RATE;
    let p0 = phase_deg.to_radians();
    for _ in 0..n {
        let s = (p0 + *phase_acc).sin();
        for (f, a) in frame.iter_mut().zip(&amps) {
            *f = a * s;
        }
        meter.push_frame(&frame);
        *phase_acc += w;
    }
}

pub fn silence(meter: &mut Meter, seconds: f64, channels: usize) {
    let n = (seconds * RATE).round() as usize;
    let frame = vec![0.0; channels];
    for _ in 0..n {
        meter.push_frame(&frame);
    }
}

/// A stereo 1 kHz tone, both channels at `dbfs`.
pub fn stereo_tone(meter: &mut Meter, seconds: f64, dbfs: f64, phase_acc: &mut f64) {
    tone(meter, seconds, 1000.0, &[dbfs, dbfs], 0.0, phase_acc);
}

pub fn assert_within(name: &str, got: f64, expected: f64, tol: f64) {
    assert!(
        (got - expected).abs() <= tol + 1e-9,
        "{name}: got {got:.3}, expected {expected:.1} ±{tol}"
    );
}

/// Asymmetric tolerance, as the true-peak cases state it: `expected + plus / - minus`.
pub fn assert_between(name: &str, got: f64, expected: f64, plus: f64, minus: f64) {
    assert!(
        got <= expected + plus + 1e-9 && got >= expected - minus - 1e-9,
        "{name}: got {got:.3}, expected {expected:.1} +{plus}/-{minus}"
    );
}
