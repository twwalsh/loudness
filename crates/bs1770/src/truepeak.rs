//! True-peak level per ITU-R BS.1770 Annex 2: oversample by 4 with the Recommendation's 48-tap
//! polyphase interpolation filter, take the absolute maximum, express it in dB TP.
//!
//! The tabulated coefficients are a detection filter, not an audio-grade one; the spec says so and
//! that is why it is only 48 taps. Annex 2 also asks for an oversampled rate of at least 192 kHz
//! before a result is labelled dB TP, so below 48 kHz two stages are cascaded (16x), which is also
//! measurably more accurate near Nyquist than a single 4x stage.

const PHASES: usize = 4;
const TAPS: usize = 12;

/// BS.1770-4 Annex 2, Attachment 1: the 48-tap filter as four polyphase branches of twelve taps,
/// `COEFFS[phase][tap]`. The prototype is linear-phase, so branch 0 mirrors branch 3 and 1 mirrors 2;
/// every value is an exact multiple of 2^-16, which the test below checks against a typo.
#[rustfmt::skip]
pub const COEFFS: [[f64; TAPS]; PHASES] = [
    [ 0.001_708_984_375_0,  0.010_986_328_125_0, -0.019_653_320_312_5,  0.033_203_125_000_0,
     -0.059_448_242_187_5,  0.137_329_101_562_5,  0.972_167_968_750_0, -0.102_294_921_875_0,
      0.047_607_421_875_0, -0.026_611_328_125_0,  0.014_892_578_125_0, -0.008_300_781_250_0],
    [-0.029_174_804_687_5,  0.029_296_875_000_0, -0.051_757_812_500_0,  0.089_111_328_125_0,
     -0.166_503_906_250_0,  0.465_087_890_625_0,  0.779_785_156_250_0, -0.200_317_382_812_5,
      0.101_562_500_000_0, -0.058_227_539_062_5,  0.033_081_054_687_5, -0.018_920_898_437_5],
    [-0.018_920_898_437_5,  0.033_081_054_687_5, -0.058_227_539_062_5,  0.101_562_500_000_0,
     -0.200_317_382_812_5,  0.779_785_156_250_0,  0.465_087_890_625_0, -0.166_503_906_250_0,
      0.089_111_328_125_0, -0.051_757_812_500_0,  0.029_296_875_000_0, -0.029_174_804_687_5],
    [-0.008_300_781_250_0,  0.014_892_578_125_0, -0.026_611_328_125_0,  0.047_607_421_875_0,
     -0.102_294_921_875_0,  0.972_167_968_750_0,  0.137_329_101_562_5, -0.059_448_242_187_5,
      0.033_203_125_000_0, -0.019_653_320_312_5,  0.010_986_328_125_0,  0.001_708_984_375_0],
];

/// Oversampled rate below which a result may not be called dB TP (Annex 2).
pub const MIN_DBTP_RATE: f64 = 192_000.0;

/// One 4x interpolating stage: a 16-slot ring (12 taps fit, the wrap is a mask).
#[derive(Clone, Debug)]
struct Stage {
    hist: [f64; 16],
    pos: usize,
}

impl Stage {
    const fn new() -> Self {
        Self {
            hist: [0.0; 16],
            pos: 0,
        }
    }

    /// Push one input sample; return the four interpolated outputs.
    #[inline]
    fn push(&mut self, x: f64) -> [f64; PHASES] {
        self.pos = (self.pos + 1) & 15;
        self.hist[self.pos] = x;
        let mut out = [0.0; PHASES];
        for (p, o) in out.iter_mut().enumerate() {
            let mut acc = 0.0;
            for (k, c) in COEFFS[p].iter().enumerate() {
                acc += c * self.hist[(self.pos.wrapping_sub(k)) & 15];
            }
            *o = acc;
        }
        out
    }
}

/// True-peak detector for one channel.
#[derive(Clone, Debug)]
pub struct TruePeak {
    stages: usize,
    first: Stage,
    second: Stage,
    peak: f64,
}

impl TruePeak {
    /// One 4x stage when that reaches 192 kHz, two (16x) below that.
    pub fn new(sample_rate: f64) -> Self {
        Self::with_stages(Self::stages_for(sample_rate))
    }

    pub fn with_stages(stages: usize) -> Self {
        assert!((1..=2).contains(&stages), "one or two stages");
        Self {
            stages,
            first: Stage::new(),
            second: Stage::new(),
            peak: 0.0,
        }
    }

    pub fn stages_for(sample_rate: f64) -> usize {
        if sample_rate * PHASES as f64 >= MIN_DBTP_RATE {
            1
        } else {
            2
        }
    }

    pub const fn oversampling(&self) -> usize {
        if self.stages == 1 {
            PHASES
        } else {
            PHASES * PHASES
        }
    }

    #[inline]
    pub fn push(&mut self, x: f64) {
        let a = self.first.push(x);
        if self.stages == 1 {
            for v in a {
                self.peak = self.peak.max(v.abs());
            }
        } else {
            for v in a {
                for w in self.second.push(v) {
                    self.peak = self.peak.max(w.abs());
                }
            }
        }
    }

    /// Linear peak so far (1.0 = full scale).
    pub const fn peak(&self) -> f64 {
        self.peak
    }

    /// Peak so far in dB TP; `-inf` for silence.
    pub fn peak_dbtp(&self) -> f64 {
        20.0 * self.peak.log10()
    }

    pub fn reset(&mut self) {
        *self = Self::with_stages(self.stages);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branches_mirror_and_quantise_to_16_bits() {
        for k in 0..TAPS {
            assert_eq!(COEFFS[0][k], COEFFS[3][TAPS - 1 - k]);
            assert_eq!(COEFFS[1][k], COEFFS[2][TAPS - 1 - k]);
        }
        for row in &COEFFS {
            for c in row {
                let scaled = c * 65_536.0;
                assert_eq!(scaled, scaled.round(), "{c} is not a multiple of 2^-16");
            }
        }
        // Each branch is an interpolation filter with DC gain near 1. The outer branches sum to 1.0016
        // and the inner ones to 0.973: the pass-band ripple of a 48-tap detection filter, which is why
        // the Recommendation's tolerances are asymmetric.
        for row in &COEFFS {
            let sum: f64 = row.iter().sum();
            assert!((sum - 1.0).abs() < 0.03, "branch sums to {sum}");
        }
    }

    /// A low-frequency full-scale sine is reconstructed almost exactly, so it reads 0 dB TP. (A DC
    /// step is the wrong probe: its leading edge overshoots through any low-pass, by +0.95 dB here.)
    #[test]
    fn a_full_scale_low_frequency_sine_reads_zero_dbtp() {
        let mut tp = TruePeak::with_stages(1);
        let n = 48_000usize;
        for i in 0..n {
            let env = (i as f64 / 480.0).min(1.0).min((n - 1 - i) as f64 / 480.0);
            tp.push(env * (2.0 * core::f64::consts::PI * 100.0 * i as f64 / 48_000.0).sin());
        }
        assert!(tp.peak_dbtp().abs() < 0.05, "{}", tp.peak_dbtp());
    }
}
