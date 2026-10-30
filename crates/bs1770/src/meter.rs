//! The loudness meter: ITU-R BS.1770 momentary, short-term and integrated loudness, the Tech 3342
//! loudness range, and true peak. File-based: every sub-block is kept, so the gating passes are exact
//! rather than histogram-quantised.
//!
//! The grid is 10 ms. Integrated gating steps 400 ms blocks every 100 ms (the 75 % overlap the
//! Recommendation specifies) and the short-term series steps every 100 ms (Tech 3342 asks for at least
//! 10 Hz), but momentary is read every 10 ms: Tech 3341 case 13 places a 400 ms tone at 20 ms offsets
//! and expects its maximum within 0.1 LU, which a 100 ms grid cannot resolve.

use crate::kweight::{KWeighting, KWeightingState};
use crate::lra::loudness_range;
use crate::truepeak::TruePeak;

/// BS.1770's calibration constant. K-weighting is +0.69 dB at 1 kHz, which this cancels, so a 1 kHz
/// sine measures its own dBFS level.
pub const OFFSET_DB: f64 = -0.691;
/// Absolute gate (BS.1770): blocks at or below this are silence.
pub const ABSOLUTE_GATE_LUFS: f64 = -70.0;
/// Relative gate, below the absolute-gated loudness.
pub const RELATIVE_GATE_LU: f64 = -10.0;
/// Reported for a measurement with nothing above the gate.
pub const SILENCE: f64 = f64::NEG_INFINITY;

const GRID_MS: u32 = 10;
const MOMENTARY_BLOCKS: usize = 40;
const SHORT_TERM_BLOCKS: usize = 300;
/// Sub-blocks per 100 ms: the step for integrated gating blocks and the short-term series.
const HOP: usize = 10;

/// BS.1770 Table 3 channel weights `G_i` for a channel count, in SMPTE order.
/// Mono and stereo are unity; 5.0 is L R C Ls Rs with the surrounds at +1.5 dB; 5.1 adds the LFE in
/// slot 3 at weight 0, as the Recommendation excludes it. Anything else is unity, which is what a
/// caller with an unusual layout should override with [`Meter::with_weights`].
pub fn default_weights(channels: usize) -> Vec<f64> {
    match channels {
        5 => vec![1.0, 1.0, 1.0, 1.41, 1.41],
        6 => vec![1.0, 1.0, 1.0, 0.0, 1.41, 1.41],
        n => vec![1.0; n],
    }
}

/// Everything a finished measurement reports.
#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    /// Programme loudness, gated per BS.1770, in LUFS.
    pub integrated_lufs: f64,
    /// Loudness range per EBU Tech 3342, in LU.
    pub loudness_range_lu: f64,
    /// Maximum momentary loudness (400 ms window), in LUFS.
    pub max_momentary_lufs: f64,
    /// Maximum short-term loudness (3 s window), in LUFS.
    pub max_short_term_lufs: f64,
    /// Maximum true-peak level over all channels, in dB TP.
    pub true_peak_dbtp: f64,
    /// Per-channel true-peak level, in dB TP.
    pub channel_true_peak_dbtp: Vec<f64>,
    /// Maximum sample peak over all channels, in dBFS.
    pub sample_peak_dbfs: f64,
    /// Oversampling factor the true-peak detector used (4 or 16).
    pub true_peak_oversampling: usize,
    pub sample_rate: f64,
    pub channels: usize,
    pub frames: u64,
}

impl Measurement {
    pub fn duration_seconds(&self) -> f64 {
        self.frames as f64 / self.sample_rate
    }
}

/// Feed it audio, then ask.
#[derive(Clone, Debug)]
pub struct Meter {
    sample_rate: f64,
    channels: usize,
    weights: Vec<f64>,
    kw: KWeighting,
    states: Vec<KWeightingState>,
    true_peaks: Vec<TruePeak>,
    sample_peak: f64,
    grid: usize,
    grid_pos: usize,
    acc: Vec<f64>,
    /// Weighted mean square of each completed 10 ms sub-block: `Σ G_i · z_i`.
    sub_blocks: Vec<f64>,
    frames: u64,
}

impl Meter {
    pub fn new(sample_rate: f64, channels: usize) -> Self {
        Self::with_weights(sample_rate, default_weights(channels))
    }

    /// Explicit `G_i` per channel, for layouts the defaults do not cover.
    pub fn with_weights(sample_rate: f64, weights: Vec<f64>) -> Self {
        assert!(sample_rate > 0.0, "sample rate");
        assert!(!weights.is_empty(), "at least one channel");
        let channels = weights.len();
        let grid = ((sample_rate * f64::from(GRID_MS) / 1000.0).round() as usize).max(1);
        Self {
            sample_rate,
            channels,
            weights,
            kw: KWeighting::new(sample_rate),
            states: vec![KWeightingState::default(); channels],
            true_peaks: vec![TruePeak::new(sample_rate); channels],
            sample_peak: 0.0,
            grid,
            grid_pos: 0,
            acc: vec![0.0; channels],
            sub_blocks: Vec::new(),
            frames: 0,
        }
    }

    pub const fn channels(&self) -> usize {
        self.channels
    }

    pub const fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// One frame: one sample per channel.
    #[inline]
    pub fn push_frame(&mut self, frame: &[f64]) {
        debug_assert_eq!(frame.len(), self.channels);
        for (ch, &x) in frame.iter().enumerate() {
            self.sample_peak = self.sample_peak.max(x.abs());
            self.true_peaks[ch].push(x);
            let y = self.kw.process(&mut self.states[ch], x);
            self.acc[ch] += y * y;
        }
        self.frames += 1;
        self.grid_pos += 1;
        if self.grid_pos == self.grid {
            let grid = self.grid as f64;
            let power: f64 = self
                .acc
                .iter()
                .zip(&self.weights)
                .map(|(a, g)| g * (a / grid))
                .sum();
            self.sub_blocks.push(power);
            self.acc.iter_mut().for_each(|a| *a = 0.0);
            self.grid_pos = 0;
        }
    }

    /// Interleaved samples, `channels` per frame. A trailing partial frame is ignored.
    pub fn push_interleaved(&mut self, samples: &[f32]) {
        let mut frame = vec![0.0; self.channels];
        for chunk in samples.chunks_exact(self.channels) {
            for (f, &s) in frame.iter_mut().zip(chunk) {
                *f = f64::from(s);
            }
            self.push_frame(&frame);
        }
    }

    /// One slice per channel, equal lengths.
    pub fn push_planar(&mut self, planes: &[&[f32]]) {
        assert_eq!(planes.len(), self.channels, "one plane per channel");
        let n = planes.iter().map(|p| p.len()).min().unwrap_or(0);
        let mut frame = vec![0.0; self.channels];
        for i in 0..n {
            for (f, p) in frame.iter_mut().zip(planes) {
                *f = f64::from(p[i]);
            }
            self.push_frame(&frame);
        }
    }

    /// Loudness in LUFS of a mean power (each power being `Σ G_i z_i`).
    fn lufs(mean_power: f64) -> f64 {
        if mean_power <= 0.0 {
            SILENCE
        } else {
            OFFSET_DB + 10.0 * mean_power.log10()
        }
    }

    /// Mean power of every complete `blocks`-sub-block window, every `step` sub-blocks, by prefix sums.
    fn window_powers(&self, blocks: usize, step: usize) -> Vec<f64> {
        let n = self.sub_blocks.len();
        if n < blocks {
            return Vec::new();
        }
        let mut prefix = Vec::with_capacity(n + 1);
        prefix.push(0.0);
        let mut acc = 0.0;
        for &v in &self.sub_blocks {
            acc += v;
            prefix.push(acc);
        }
        (blocks..=n)
            .step_by(step)
            .map(|end| (prefix[end] - prefix[end - blocks]) / blocks as f64)
            .collect()
    }

    /// Momentary loudness (400 ms) at every 10 ms step, from the first complete window.
    pub fn momentary_series(&self) -> Vec<f64> {
        self.window_powers(MOMENTARY_BLOCKS, 1)
            .into_iter()
            .map(Self::lufs)
            .collect()
    }

    /// Short-term loudness (3 s) at every 100 ms step, from the first complete window.
    pub fn short_term_series(&self) -> Vec<f64> {
        self.window_powers(SHORT_TERM_BLOCKS, HOP)
            .into_iter()
            .map(Self::lufs)
            .collect()
    }

    /// Integrated loudness with BS.1770 gating: 400 ms blocks at 75 % overlap, blocks above -70 LUFS,
    /// then blocks above the relative threshold (10 LU under the mean of the first set), and the mean
    /// of what survives. Means are of power, never of decibels.
    pub fn integrated_lufs(&self) -> f64 {
        if self.sub_blocks.len() < MOMENTARY_BLOCKS {
            return SILENCE;
        }
        let block_powers = self.window_powers(MOMENTARY_BLOCKS, HOP);
        let above_absolute: Vec<f64> = block_powers
            .iter()
            .copied()
            .filter(|&p| Self::lufs(p) > ABSOLUTE_GATE_LUFS)
            .collect();
        if above_absolute.is_empty() {
            return SILENCE;
        }
        let abs_gated = Self::lufs(mean(&above_absolute));
        let relative_gate = abs_gated + RELATIVE_GATE_LU;
        let above_relative: Vec<f64> = above_absolute
            .into_iter()
            .filter(|&p| Self::lufs(p) > relative_gate)
            .collect();
        if above_relative.is_empty() {
            return SILENCE;
        }
        Self::lufs(mean(&above_relative))
    }

    pub fn loudness_range_lu(&self) -> f64 {
        loudness_range(&self.short_term_series())
    }

    pub fn finish(&self) -> Measurement {
        let per_channel: Vec<f64> = self.true_peaks.iter().map(TruePeak::peak_dbtp).collect();
        let max_of = |v: &[f64]| v.iter().copied().fold(SILENCE, f64::max);
        Measurement {
            integrated_lufs: self.integrated_lufs(),
            loudness_range_lu: self.loudness_range_lu(),
            max_momentary_lufs: max_of(&self.momentary_series()),
            max_short_term_lufs: max_of(&self.short_term_series()),
            true_peak_dbtp: max_of(&per_channel),
            channel_true_peak_dbtp: per_channel,
            sample_peak_dbfs: 20.0 * self.sample_peak.log10(),
            true_peak_oversampling: self.true_peaks[0].oversampling(),
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames: self.frames,
        }
    }
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}
