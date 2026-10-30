//! ITU-R BS.1770 loudness and true peak, and the EBU R 128 loudness range, with no dependencies.
//!
//! ```
//! use bs1770::Meter;
//! let mut meter = Meter::new(48_000.0, 2);
//! // a 1 kHz stereo sine at -23 dBFS per channel, 20 s: EBU Tech 3341 test case 1
//! let amp = 10f64.powf(-23.0 / 20.0);
//! for i in 0..(48_000 * 20) {
//!     let x = amp * (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 48_000.0).sin();
//!     meter.push_frame(&[x, x]);
//! }
//! let m = meter.finish();
//! assert!((m.integrated_lufs + 23.0).abs() < 0.1);
//! ```
//!
//! Conformance: the crate's integration tests synthesise EBU Tech 3341 (loudness and true peak) and
//! Tech 3342 (loudness range) minimum-requirements signals from their specifications and check the
//! published expected values; the EBU's own files run too when `EBU_TEST_SET_DIR` points at them.

#![forbid(unsafe_code)]

pub mod kweight;
pub mod lra;
pub mod meter;
pub mod truepeak;

pub use lra::loudness_range;
pub use meter::{
    default_weights, Measurement, Meter, ABSOLUTE_GATE_LUFS, OFFSET_DB, RELATIVE_GATE_LU, SILENCE,
};
pub use truepeak::TruePeak;
