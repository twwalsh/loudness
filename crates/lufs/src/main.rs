//! `lufs <file.wav> [--json]`: measure a WAV file and print the EBU R 128 numbers.
use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;

use bs1770::{Measurement, Meter};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let files: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    if files.is_empty() {
        eprintln!("usage: lufs <file.wav>... [--json]");
        return ExitCode::from(2);
    }
    let mut failed = false;
    for f in files {
        match measure(Path::new(f)) {
            Ok(m) => {
                if json {
                    println!("{}", to_json(f, &m));
                } else {
                    print_table(f, &m);
                }
            }
            Err(e) => {
                eprintln!("{f}: {e}");
                failed = true;
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn measure(path: &Path) -> Result<Measurement, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let mut meter = Meter::new(f64::from(spec.sample_rate), spec.channels as usize);
    let channels = spec.channels as usize;
    let mut frame = vec![0.0f64; channels];
    let mut i = 0;
    match spec.sample_format {
        hound::SampleFormat::Float => {
            for s in reader.samples::<f32>() {
                frame[i] = f64::from(s.map_err(|e| e.to_string())?);
                i += 1;
                if i == channels {
                    meter.push_frame(&frame);
                    i = 0;
                }
            }
        }
        hound::SampleFormat::Int => {
            let scale = 2f64.powi(i32::from(spec.bits_per_sample) - 1);
            for s in reader.samples::<i32>() {
                frame[i] = f64::from(s.map_err(|e| e.to_string())?) / scale;
                i += 1;
                if i == channels {
                    meter.push_frame(&frame);
                    i = 0;
                }
            }
        }
    }
    Ok(meter.finish())
}

fn db(v: f64) -> String {
    if v.is_finite() {
        format!("{v:7.1}")
    } else {
        "   -inf".to_string()
    }
}

fn print_table(name: &str, m: &Measurement) {
    println!("{name}");
    println!("  {:<22}{} LUFS", "Integrated", db(m.integrated_lufs));
    println!("  {:<22}{} LU", "Loudness range", db(m.loudness_range_lu));
    println!("  {:<22}{} LUFS", "Max momentary", db(m.max_momentary_lufs));
    println!("  {:<22}{} LUFS", "Max short-term", db(m.max_short_term_lufs));
    println!(
        "  {:<22}{} dB TP  ({}x oversampled{})",
        "True peak",
        db(m.true_peak_dbtp),
        m.true_peak_oversampling,
        if m.channel_true_peak_dbtp.len() > 1 {
            format!(
                "; per channel {}",
                m.channel_true_peak_dbtp
                    .iter()
                    .map(|v| db(*v).trim().to_string())
                    .collect::<Vec<_>>()
                    .join(" / ")
            )
        } else {
            String::new()
        }
    );
    println!("  {:<22}{} dBFS", "Sample peak", db(m.sample_peak_dbfs));
    println!(
        "  {:<22}{:.0} Hz, {} ch, {:.2} s",
        "Format",
        m.sample_rate,
        m.channels,
        m.duration_seconds()
    );
}

fn to_json(name: &str, m: &Measurement) -> String {
    let num = |v: f64| {
        if v.is_finite() {
            format!("{v:.2}")
        } else {
            "null".to_string()
        }
    };
    format!(
        concat!(
            "{{\"file\":{},\"integrated_lufs\":{},\"loudness_range_lu\":{},\"max_momentary_lufs\":{},",
            "\"max_short_term_lufs\":{},\"true_peak_dbtp\":{},\"channel_true_peak_dbtp\":[{}],",
            "\"sample_peak_dbfs\":{},\"true_peak_oversampling\":{},\"sample_rate\":{},\"channels\":{},\"frames\":{}}}"
        ),
        json_string(name),
        num(m.integrated_lufs),
        num(m.loudness_range_lu),
        num(m.max_momentary_lufs),
        num(m.max_short_term_lufs),
        num(m.true_peak_dbtp),
        m.channel_true_peak_dbtp.iter().map(|v| num(*v)).collect::<Vec<_>>().join(","),
        num(m.sample_peak_dbfs),
        m.true_peak_oversampling,
        m.sample_rate,
        m.channels,
        m.frames
    )
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
