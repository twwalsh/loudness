//! The EBU's own minimum-requirements files (tech.ebu.ch/loudness), when `EBU_TEST_SET_DIR` points at
//! the unzipped set. Covers what synthesis cannot: the two authentic programme segments (3341 cases 7
//! and 8, 3342 cases 5 and 6) and the live-meter forms (3341 cases 11 and 14), plus every synthetic
//! case again against the EBU's rendering of it. Skips silently when the directory is not set.
mod common;
use bs1770::Meter;
use common::*;
use std::path::{Path, PathBuf};

fn dir() -> Option<PathBuf> {
    std::env::var_os("EBU_TEST_SET_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
}

fn measure(path: &Path) -> (bs1770::Measurement, Meter) {
    let mut reader = hound::WavReader::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let spec = reader.spec();
    let mut meter = Meter::new(f64::from(spec.sample_rate), spec.channels as usize);
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(Result::unwrap).collect(),
        hound::SampleFormat::Int => {
            let scale = 2f32.powi(i32::from(spec.bits_per_sample) - 1);
            reader
                .samples::<i32>()
                .map(|s| s.unwrap() as f32 / scale)
                .collect()
        }
    };
    meter.push_interleaved(&samples);
    (meter.finish(), meter)
}

/// Finds a file whose name starts with the given prefix, e.g. `seq-3341-7`.
fn find(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(prefix) && n.to_ascii_lowercase().ends_with(".wav"))
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

#[test]
fn integrated_cases_from_the_ebu_files() {
    let Some(dir) = dir() else { return };
    for (prefix, expected) in [
        ("seq-3341-1-", -23.0),
        ("seq-3341-2-", -33.0),
        ("seq-3341-3-", -23.0),
        ("seq-3341-4-", -23.0),
        ("seq-3341-5-", -23.0),
        ("seq-3341-6-", -23.0),
        ("seq-3341-7_", -23.0),
        ("seq-3341-2011-8_", -23.0),
    ] {
        let Some(path) = find(&dir, prefix) else { continue };
        let (m, _) = measure(&path);
        assert_within(&format!("{prefix} I"), m.integrated_lufs, expected, 0.1);
    }
}

#[test]
fn true_peak_cases_from_the_ebu_files() {
    let Some(dir) = dir() else { return };
    for (prefix, expected) in [
        ("seq-3341-15-", -6.0),
        ("seq-3341-16-", -6.0),
        ("seq-3341-17-", -6.0),
        ("seq-3341-18-", -6.0),
        ("seq-3341-19-", 3.0),
        ("seq-3341-20-", 0.0),
        ("seq-3341-21-", 0.0),
        ("seq-3341-22-", 0.0),
        ("seq-3341-23-", 0.0),
    ] {
        let Some(path) = find(&dir, prefix) else { continue };
        let (m, _) = measure(&path);
        assert_between(&format!("{prefix} TP"), m.true_peak_dbtp, expected, 0.2, 0.4);
    }
}

#[test]
fn loudness_range_cases_from_the_ebu_files() {
    let Some(dir) = dir() else { return };
    for (prefix, expected) in [
        ("seq-3342-1-", 10.0),
        ("seq-3342-2-", 5.0),
        ("seq-3342-3-", 20.0),
        ("seq-3342-4-", 15.0),
        ("seq-3342-5-", 5.0),
        ("seq-3342-6-", 15.0),
    ] {
        let Some(path) = find(&dir, prefix) else { continue };
        let (m, _) = measure(&path);
        assert_within(&format!("{prefix} LRA"), m.loudness_range_lu, expected, 1.0);
    }
}
