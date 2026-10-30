//! EBU Tech 3341 Table 1, synthesised from the table's own wording at 48 kHz.
//! Cases 7, 8 (authentic programme) and 11, 14 (live-meter forms) are in `ebu_files.rs`.
mod common;
use bs1770::Meter;
use common::*;
use std::f64::consts::PI;

#[test]
fn case_1_stereo_1khz_minus_23() {
    let mut m = Meter::new(RATE, 2);
    stereo_tone(&mut m, 20.0, -23.0, &mut 0.0);
    let r = m.finish();
    assert_within("I", r.integrated_lufs, -23.0, 0.1);
    assert_within("max M", r.max_momentary_lufs, -23.0, 0.1);
    assert_within("max S", r.max_short_term_lufs, -23.0, 0.1);
    // Every momentary and short-term reading of a steady tone is the same number.
    for v in m.momentary_series().iter().chain(m.short_term_series().iter()) {
        assert_within("M/S", *v, -23.0, 0.1);
    }
}

#[test]
fn case_2_stereo_1khz_minus_33() {
    let mut m = Meter::new(RATE, 2);
    stereo_tone(&mut m, 20.0, -33.0, &mut 0.0);
    let r = m.finish();
    assert_within("I", r.integrated_lufs, -33.0, 0.1);
    assert_within("max M", r.max_momentary_lufs, -33.0, 0.1);
    assert_within("max S", r.max_short_term_lufs, -33.0, 0.1);
}

#[test]
fn case_3_relative_gate_drops_the_quiet_tones() {
    let mut m = Meter::new(RATE, 2);
    let mut p = 0.0;
    stereo_tone(&mut m, 10.0, -36.0, &mut p);
    stereo_tone(&mut m, 60.0, -23.0, &mut p);
    stereo_tone(&mut m, 10.0, -36.0, &mut p);
    assert_within("I", m.integrated_lufs(), -23.0, 0.1);
}

#[test]
fn case_4_absolute_and_relative_gates() {
    let mut m = Meter::new(RATE, 2);
    let mut p = 0.0;
    stereo_tone(&mut m, 10.0, -72.0, &mut p);
    stereo_tone(&mut m, 10.0, -36.0, &mut p);
    stereo_tone(&mut m, 60.0, -23.0, &mut p);
    stereo_tone(&mut m, 10.0, -36.0, &mut p);
    stereo_tone(&mut m, 10.0, -72.0, &mut p);
    assert_within("I", m.integrated_lufs(), -23.0, 0.1);
}

#[test]
fn case_5_a_louder_middle_keeps_the_surround() {
    let mut m = Meter::new(RATE, 2);
    let mut p = 0.0;
    stereo_tone(&mut m, 20.0, -26.0, &mut p);
    stereo_tone(&mut m, 20.1, -20.0, &mut p);
    stereo_tone(&mut m, 20.0, -26.0, &mut p);
    assert_within("I", m.integrated_lufs(), -23.0, 0.1);
}

#[test]
fn case_6_five_channel_weights() {
    // L R C Ls Rs at -28 -28 -24 -30 -30 dBFS: the surround +1.5 dB weight brings the sum to -23.
    let mut m = Meter::new(RATE, 5);
    tone(
        &mut m,
        20.0,
        1000.0,
        &[-28.0, -28.0, -24.0, -30.0, -30.0],
        0.0,
        &mut 0.0,
    );
    assert_within("I", m.integrated_lufs(), -23.0, 0.1);
}

#[test]
fn case_9_short_term_settles_after_three_seconds() {
    let mut m = Meter::new(RATE, 2);
    let mut p = 0.0;
    for _ in 0..5 {
        stereo_tone(&mut m, 1.34, -20.0, &mut p);
        stereo_tone(&mut m, 1.66, -30.0, &mut p);
    }
    let s = m.short_term_series();
    // The series starts at t = 3 s by construction (first complete window), so every value counts.
    for (i, v) in s.iter().enumerate() {
        assert_within(&format!("S at step {i}"), *v, -23.0, 0.1);
    }
}

#[test]
fn case_10_max_short_term_per_segment() {
    for i in 0..20usize {
        let mut m = Meter::new(RATE, 2);
        silence(&mut m, i as f64 * 0.15, 2);
        stereo_tone(&mut m, 3.0, -23.0, &mut 0.0);
        silence(&mut m, 1.0, 2);
        assert_within(
            &format!("segment {i} max S"),
            m.finish().max_short_term_lufs,
            -23.0,
            0.1,
        );
    }
}

#[test]
fn case_12_momentary_settles_after_one_second() {
    let mut m = Meter::new(RATE, 2);
    let mut p = 0.0;
    for _ in 0..25 {
        stereo_tone(&mut m, 0.18, -20.0, &mut p);
        stereo_tone(&mut m, 0.22, -30.0, &mut p);
    }
    let series = m.momentary_series();
    // Steps are 10 ms; the first complete window ends at 0.4 s, so step k ends at 0.4 + 0.01k s.
    for (k, v) in series.iter().enumerate() {
        let t = 0.4 + 0.01 * k as f64;
        if t >= 1.0 {
            assert_within(&format!("M at {t:.1}s"), *v, -23.0, 0.1);
        }
    }
}

#[test]
fn case_13_max_momentary_per_segment() {
    for i in 0..20usize {
        let mut m = Meter::new(RATE, 2);
        silence(&mut m, i as f64 * 0.020, 2);
        stereo_tone(&mut m, 0.4, -23.0, &mut 0.0);
        silence(&mut m, 1.0, 2);
        assert_within(
            &format!("segment {i} max M"),
            m.finish().max_momentary_lufs,
            -23.0,
            0.1,
        );
    }
}

/// Cases 15-19: stereo sines at fs/4, fs/6, fs/8 with the stated phases, 10 ms fades, amplitude 0.50
/// (or 1.41 for 19). Expected true peak -6.0 (+3.0) dB TP, +0.2/-0.4.
fn true_peak_case(name: &str, divisor: f64, phase_deg: f64, amplitude: f64, expected: f64) {
    let mut m = Meter::new(RATE, 2);
    let hz = RATE / divisor;
    let n = (RATE * 2.0) as usize;
    let fade = (RATE * 0.010) as usize;
    let w = 2.0 * PI * hz / RATE;
    for i in 0..n {
        let env = if i < fade {
            i as f64 / fade as f64
        } else if i >= n - fade {
            (n - 1 - i) as f64 / fade as f64
        } else {
            1.0
        };
        let x = amplitude * env * (phase_deg.to_radians() + w * i as f64).sin();
        m.push_frame(&[x, x]);
    }
    assert_between(name, m.finish().true_peak_dbtp, expected, 0.2, 0.4);
}

#[test]
fn case_15_fs_over_4_phase_0() {
    true_peak_case("case 15", 4.0, 0.0, 0.5, -6.0);
}
#[test]
fn case_16_fs_over_4_phase_45() {
    true_peak_case("case 16", 4.0, 45.0, 0.5, -6.0);
}
#[test]
fn case_17_fs_over_6_phase_60() {
    true_peak_case("case 17", 6.0, 60.0, 0.5, -6.0);
}
#[test]
fn case_18_fs_over_8_phase_67_5() {
    true_peak_case("case 18", 8.0, 67.5, 0.5, -6.0);
}
#[test]
fn case_19_fs_over_4_phase_45_amplitude_1_41() {
    true_peak_case("case 19", 4.0, 45.0, 1.41, 3.0);
}

/// Cases 20-23: a fs/6 tone at 0.5 containing one period of a fs/4 tone at amplitude 1.0, synthesised
/// at 4*fs, low-pass filtered and decimated with a 0..3 sample offset. Expected 0.0 dB TP, +0.2/-0.4.
fn inter_sample_case(offset: usize) {
    let fs4 = RATE * 4.0;
    let total = (fs4 * 1.0) as usize; // 1 s at 192 kHz
    let f_carrier = RATE / 6.0;
    let f_burst = RATE / 4.0;
    // Build the high-rate signal: carrier everywhere except one burst period in the middle, continuous
    // in phase at both joins (both tones start their period at the join).
    let mut hi = vec![0.0f64; total];
    let burst_len = (fs4 / f_burst).round() as usize; // one period of fs/4 at 4fs = 16 samples
    let mid = total / 2;
    let fade = (fs4 * 0.010) as usize;
    for (i, v) in hi.iter_mut().enumerate() {
        let env = if i < fade {
            i as f64 / fade as f64
        } else if i >= total - fade {
            (total - 1 - i) as f64 / fade as f64
        } else {
            1.0
        };
        *v = if (mid..mid + burst_len).contains(&i) {
            // The burst starts where the carrier crosses zero rising: phase 0 at i = mid for both.
            (2.0 * PI * f_burst * (i - mid) as f64 / fs4).sin()
        } else {
            // Carrier phased so it is at zero at i = mid.
            0.5 * env * (2.0 * PI * f_carrier * (i as f64 - mid as f64) / fs4).sin()
        };
    }
    // Anti-alias low-pass: windowed sinc at fs/2 (cutoff 24 kHz at 192 kHz), 255 taps.
    let taps = 255usize;
    let fc = 0.5 / 4.0; // cycles per high-rate sample
    let half = (taps / 2) as f64;
    let h: Vec<f64> = (0..taps)
        .map(|k| {
            let t = k as f64 - half;
            let sinc = if t == 0.0 {
                2.0 * fc
            } else {
                (2.0 * PI * fc * t).sin() / (PI * t)
            };
            let win = 0.42 - 0.5 * (2.0 * PI * k as f64 / (taps - 1) as f64).cos()
                + 0.08 * (4.0 * PI * k as f64 / (taps - 1) as f64).cos();
            sinc * win
        })
        .collect();
    let gain: f64 = h.iter().sum();
    let mut m = Meter::new(RATE, 2);
    let mut i = offset + taps;
    while i < total {
        let mut acc = 0.0;
        for (k, c) in h.iter().enumerate() {
            acc += c * hi[i - k];
        }
        let x = acc / gain;
        m.push_frame(&[x, x]);
        i += 4;
    }
    assert_between(
        &format!("case {}", 20 + offset),
        m.finish().true_peak_dbtp,
        0.0,
        0.2,
        0.4,
    );
}

#[test]
fn case_20_inter_sample_peak_offset_0() {
    inter_sample_case(0);
}
#[test]
fn case_21_inter_sample_peak_offset_1() {
    inter_sample_case(1);
}
#[test]
fn case_22_inter_sample_peak_offset_2() {
    inter_sample_case(2);
}
#[test]
fn case_23_inter_sample_peak_offset_3() {
    inter_sample_case(3);
}

/// Tech 3341 §2.9: a 1 kHz stereo sine at -18 dBFS reads -18.0 LUFS.
#[test]
fn calibration_minus_18_dbfs_reads_minus_18_lufs() {
    let mut m = Meter::new(RATE, 2);
    stereo_tone(&mut m, 5.0, -18.0, &mut 0.0);
    assert_within("I", m.integrated_lufs(), -18.0, 0.1);
}

/// The same signal at other rates must measure the same: the K-filter is regenerated per rate.
#[test]
fn case_1_holds_at_44_1_96_and_192_khz() {
    for rate in [44_100.0, 96_000.0, 192_000.0] {
        let mut m = Meter::new(rate, 2);
        let a = amp(-23.0);
        let n = (rate * 10.0) as usize;
        for i in 0..n {
            let x = a * (2.0 * PI * 1000.0 * i as f64 / rate).sin();
            m.push_frame(&[x, x]);
        }
        assert_within(&format!("I at {rate}"), m.integrated_lufs(), -23.0, 0.1);
    }
}
