//! The meter for the browser: decode with the Web Audio API, hand the channels here, read the numbers.
//! Nothing leaves the page.
#![forbid(unsafe_code)]

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Meter {
    inner: bs1770::Meter,
}

#[wasm_bindgen]
impl Meter {
    /// `channels` planes of equal length are pushed with [`push_planar`]; weights follow BS.1770 for
    /// 1, 2, 5 and 6 channels and are unity otherwise.
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: f64, channels: usize) -> Self {
        Self {
            inner: bs1770::Meter::new(sample_rate, channels),
        }
    }

    /// Interleaved frames.
    pub fn push_interleaved(&mut self, samples: &[f32]) {
        self.inner.push_interleaved(samples);
    }

    /// Planar audio as one buffer: the `AudioBuffer`'s channels concatenated into a single
    /// `Float32Array` of `frames * channels` samples, channel 0 first. That is one `copyFromChannel`
    /// per channel in JS and no interleaving.
    pub fn push_planar_concat(&mut self, planes: &[f32], frames: usize) {
        let channels = self.inner.channels();
        if frames == 0 || planes.len() < frames * channels {
            return;
        }
        let slices: Vec<&[f32]> = (0..channels)
            .map(|c| &planes[c * frames..(c + 1) * frames])
            .collect();
        self.inner.push_planar(&slices);
    }

    pub fn finish(&self) -> Measurement {
        let m = self.inner.finish();
        Measurement {
            integrated_lufs: m.integrated_lufs,
            loudness_range_lu: m.loudness_range_lu,
            max_momentary_lufs: m.max_momentary_lufs,
            max_short_term_lufs: m.max_short_term_lufs,
            true_peak_dbtp: m.true_peak_dbtp,
            sample_peak_dbfs: m.sample_peak_dbfs,
            true_peak_oversampling: m.true_peak_oversampling,
            channels: m.channels,
            frames: m.frames as f64,
            channel_true_peak_dbtp: m.channel_true_peak_dbtp,
        }
    }

    /// Short-term loudness at 100 ms steps, for a graph.
    pub fn short_term_series(&self) -> Vec<f64> {
        self.inner.short_term_series()
    }

    /// Momentary loudness at 100 ms steps.
    pub fn momentary_series(&self) -> Vec<f64> {
        self.inner.momentary_series()
    }
}

#[wasm_bindgen(getter_with_clone)]
pub struct Measurement {
    pub integrated_lufs: f64,
    pub loudness_range_lu: f64,
    pub max_momentary_lufs: f64,
    pub max_short_term_lufs: f64,
    pub true_peak_dbtp: f64,
    pub sample_peak_dbfs: f64,
    pub true_peak_oversampling: usize,
    pub channels: usize,
    pub frames: f64,
    pub channel_true_peak_dbtp: Vec<f64>,
}

/// Crate version, for the page footer.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
