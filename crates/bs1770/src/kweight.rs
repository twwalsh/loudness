//! The K-weighting filter of ITU-R BS.1770 Annex 1: a high shelf modelling the acoustic effect of
//! the head, then the RLB high-pass. Two double-precision biquads per channel.
//!
//! The Recommendation tabulates coefficients for 48 kHz only and says other rates "will require
//! different coefficient values, which should be chosen to provide the same frequency response".
//! Reusing the 48 kHz table at 44.1 kHz is the most common loudness-meter bug. The constants below
//! are the analog prototype that regenerates the published 48 kHz table to within f64 rounding, so the
//! filter is correct at any sample rate. Stage 1 is a Zoelzer-form shelf, not the RBJ cookbook shelf,
//! which with the same parameters misses the table by about 5e-2 per coefficient.

/// Coefficients of one Direct-Form-I biquad, normalised so `a0 = 1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Biquad {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

/// Per-channel filter memory for one biquad.
#[derive(Clone, Copy, Debug, Default)]
pub struct BiquadState {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    #[inline]
    pub fn process(&self, s: &mut BiquadState, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * s.x1 + self.b2 * s.x2 - self.a1 * s.y1 - self.a2 * s.y2;
        s.x2 = s.x1;
        s.x1 = x;
        s.y2 = s.y1;
        s.y1 = y;
        y
    }
}

const SHELF_HZ: f64 = 1_681.974_450_955_533;
const SHELF_DB: f64 = 3.999_843_853_973_347;
const SHELF_Q: f64 = 0.707_175_236_955_419_6;
const SHELF_VB_EXP: f64 = 0.499_666_774_154_541_6;
const RLB_HZ: f64 = 38.135_470_876_024_44;
const RLB_Q: f64 = 0.500_327_037_323_877_3;

/// The two K-weighting stages for a sample rate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KWeighting {
    pub shelf: Biquad,
    pub rlb: Biquad,
}

impl KWeighting {
    pub fn new(sample_rate: f64) -> Self {
        let k = (core::f64::consts::PI * SHELF_HZ / sample_rate).tan();
        let vh = 10f64.powf(SHELF_DB / 20.0);
        let vb = vh.powf(SHELF_VB_EXP);
        let a0 = 1.0 + k / SHELF_Q + k * k;
        let shelf = Biquad {
            b0: (vh + vb * k / SHELF_Q + k * k) / a0,
            b1: 2.0 * (k * k - vh) / a0,
            b2: (vh - vb * k / SHELF_Q + k * k) / a0,
            a1: 2.0 * (k * k - 1.0) / a0,
            a2: (1.0 - k / SHELF_Q + k * k) / a0,
        };
        let k = (core::f64::consts::PI * RLB_HZ / sample_rate).tan();
        let d = 1.0 + k / RLB_Q + k * k;
        let rlb = Biquad {
            b0: 1.0,
            b1: -2.0,
            b2: 1.0,
            a1: 2.0 * (k * k - 1.0) / d,
            a2: (1.0 - k / RLB_Q + k * k) / d,
        };
        Self { shelf, rlb }
    }
}

/// Filter state for one channel.
#[derive(Clone, Copy, Debug, Default)]
pub struct KWeightingState {
    shelf: BiquadState,
    rlb: BiquadState,
}

impl KWeighting {
    #[inline]
    pub fn process(&self, s: &mut KWeightingState, x: f64) -> f64 {
        self.rlb.process(&mut s.rlb, self.shelf.process(&mut s.shelf, x))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BS.1770-4 Table 1 and Table 2: the published 48 kHz coefficients.
    #[test]
    fn regenerates_the_published_48k_tables() {
        let k = KWeighting::new(48_000.0);
        // The Recommendation prints 14 decimals; the prototype reproduces them to f64 rounding.
        let close = |name: &str, a: f64, b: f64| assert!((a - b).abs() < 1e-12, "{name}: {a} vs {b}");
        close("shelf b0", k.shelf.b0, 1.535_124_859_586_97);
        close("shelf b1", k.shelf.b1, -2.691_696_189_406_38);
        close("shelf b2", k.shelf.b2, 1.198_392_810_852_85);
        close("shelf a1", k.shelf.a1, -1.690_659_293_182_41);
        close("shelf a2", k.shelf.a2, 0.732_480_774_215_85);
        close("rlb a1", k.rlb.a1, -1.990_047_454_833_98);
        close("rlb a2", k.rlb.a2, 0.990_072_250_366_21);
    }

    /// At 1 kHz the shelf's +0.69 dB cancels the -0.691 dB calibration constant: the reason a
    /// 1 kHz sine measures its own dBFS level. The bilinear transform is pre-warped at each stage's
    /// corner, so the 1 kHz gain drifts by a few hundredths of a dB between 48 and 192 kHz; the
    /// conformance tolerance is 0.1.
    #[test]
    fn one_khz_gain_is_about_plus_0_69_db() {
        for rate in [44_100.0, 48_000.0, 96_000.0, 192_000.0] {
            let k = KWeighting::new(rate);
            let mut s = KWeightingState::default();
            let n = (rate * 2.0) as usize;
            let mut sum = 0.0;
            let mut count = 0usize;
            for i in 0..n {
                let x = (2.0 * core::f64::consts::PI * 1000.0 * i as f64 / rate).sin();
                let y = k.process(&mut s, x);
                if i >= n / 2 {
                    sum += y * y;
                    count += 1;
                }
            }
            let gain_db = 10.0 * (sum / count as f64 / 0.5).log10();
            assert!((gain_db - 0.691).abs() < 0.05, "rate {rate}: {gain_db:.3} dB");
        }
    }
}
