//! EBU Tech 3342 Table 1, loudness range, synthesised at 48 kHz. Cases 5 and 6 (authentic programme)
//! are in `ebu_files.rs`.
mod common;
use bs1770::Meter;
use common::*;

fn two_tones(a: f64, b: f64) -> f64 {
    let mut m = Meter::new(RATE, 2);
    let mut p = 0.0;
    stereo_tone(&mut m, 20.0, a, &mut p);
    stereo_tone(&mut m, 20.0, b, &mut p);
    m.loudness_range_lu()
}

#[test]
fn case_1_minus_20_then_minus_30_is_10_lu() {
    assert_within("LRA", two_tones(-20.0, -30.0), 10.0, 1.0);
}

#[test]
fn case_2_minus_20_then_minus_15_is_5_lu() {
    assert_within("LRA", two_tones(-20.0, -15.0), 5.0, 1.0);
}

#[test]
fn case_3_minus_40_then_minus_20_is_20_lu() {
    assert_within("LRA", two_tones(-40.0, -20.0), 20.0, 1.0);
}

#[test]
fn case_4_five_segments_is_15_lu() {
    let mut m = Meter::new(RATE, 2);
    let mut p = 0.0;
    for level in [-50.0, -35.0, -20.0, -35.0, -50.0] {
        stereo_tone(&mut m, 20.0, level, &mut p);
    }
    assert_within("LRA", m.loudness_range_lu(), 15.0, 1.0);
}

/// "The expected response is unchanged if the test signal is repeated."
#[test]
fn case_1_repeated_is_still_10_lu() {
    let mut m = Meter::new(RATE, 2);
    let mut p = 0.0;
    for _ in 0..3 {
        stereo_tone(&mut m, 20.0, -20.0, &mut p);
        stereo_tone(&mut m, 20.0, -30.0, &mut p);
    }
    assert_within("LRA", m.loudness_range_lu(), 10.0, 1.0);
}
