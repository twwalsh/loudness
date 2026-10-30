//! Loudness range per EBU Tech 3342, as its reference implementation defines it: short-term loudness
//! (3 s windows at 10 Hz or better), an absolute gate at -70 LUFS, a relative gate 20 LU below the
//! absolute-gated level, and the distance between the 10th and 95th percentiles of what remains.

const ABSOLUTE_GATE_LUFS: f64 = -70.0;
const RELATIVE_GATE_LU: f64 = -20.0;
const LOW_PERCENTILE: f64 = 10.0;
const HIGH_PERCENTILE: f64 = 95.0;

/// `short_term` is the short-term loudness series in LUFS. Returns 0 when fewer than two values survive
/// the gates, which is also what the reference implementation yields for silence.
pub fn loudness_range(short_term: &[f64]) -> f64 {
    let abs_gated: Vec<f64> = short_term
        .iter()
        .copied()
        .filter(|&l| l >= ABSOLUTE_GATE_LUFS)
        .collect();
    if abs_gated.is_empty() {
        return 0.0;
    }
    // Mean of power, back to dB: the "undo 10log10, calculate mean" line of the MATLAB.
    let power = abs_gated.iter().map(|l| 10f64.powf(l / 10.0)).sum::<f64>() / abs_gated.len() as f64;
    let integrated = 10.0 * power.log10();
    let mut rel_gated: Vec<f64> = abs_gated
        .into_iter()
        .filter(|&l| l >= integrated + RELATIVE_GATE_LU)
        .collect();
    if rel_gated.len() < 2 {
        return 0.0;
    }
    rel_gated.sort_by(f64::total_cmp);
    let n = rel_gated.len();
    // MATLAB: sorted(round((n-1)*p/100 + 1)), 1-based; round is half away from zero, as f64::round is.
    let index = |p: f64| (((n - 1) as f64) * p / 100.0).round() as usize;
    rel_gated[index(HIGH_PERCENTILE)] - rel_gated[index(LOW_PERCENTILE)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_equal_halves_ten_lu_apart_measure_ten() {
        let mut series = vec![-20.0; 200];
        series.extend(vec![-30.0; 200]);
        assert!((loudness_range(&series) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn silence_gates_out() {
        let mut series = vec![-20.0; 200];
        series.extend(vec![-90.0; 2000]);
        assert!(loudness_range(&series).abs() < 1e-9);
    }

    #[test]
    fn a_short_loud_event_does_not_dominate() {
        let mut series = vec![-23.0; 1000];
        series.extend(vec![-5.0; 10]); // 1 % of the windows
        assert!(loudness_range(&series) < 0.5);
    }
}
