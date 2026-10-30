# loudness: ITU-R BS.1770 and EBU R 128 metering in Rust

A loudness and true-peak meter with no dependencies, built to the specifications and checked against
their conformance tests: integrated, momentary and short-term loudness (ITU-R BS.1770), loudness
range (EBU Tech 3342), and true peak (BS.1770 Annex 2). It ships as a library, a command-line tool
and a WebAssembly module that runs in the browser at
[twwalsh.com/loudness](https://twwalsh.com/loudness/), where nothing leaves the page.

```
$ lufs mix.wav
mix.wav
  Integrated              -14.1 LUFS
  Loudness range            6.3 LU
  Max momentary            -9.8 LUFS
  Max short-term          -11.2 LUFS
  True peak                -0.8 dB TP  (4x oversampled; per channel -0.8 / -1.1)
  Sample peak              -1.0 dBFS
  Format                  48000 Hz, 2 ch, 214.33 s
```

## Why it exists

I spent twenty years mastering records before and alongside engineering management, and the plugins I
sell at [fruitlessresearch.com](https://fruitlessresearch.com) carry a BS.1770 true-peak detector and
a K-weighted loudness meter in C++. This is that measurement core written again in Rust, on its own,
with the conformance tests in the repository rather than in a lab notebook. It is the first Rust I
have shipped, and the point of choosing this problem was that correctness is checkable: the EBU
publishes the test signals and the expected numbers.

## Conformance

`cargo test` synthesises the EBU minimum-requirements signals from the wording of
[Tech 3341](https://tech.ebu.ch/docs/tech/tech3341.pdf) Table 1 and
[Tech 3342](https://tech.ebu.ch/docs/tech/tech3342.pdf) Table 1 and asserts the published expected
values and tolerances:

| Cases | What they check | Result |
|---|---|---|
| 3341 1-6 | integrated loudness: calibration, absolute and relative gating, 5.0 channel weights | -23.0 ±0.1 LUFS, all pass |
| 3341 9, 10, 12, 13 | short-term and momentary windows, maxima per segment at 150 ms and 20 ms offsets | ±0.1 LU, all pass |
| 3341 15-19 | true peak of fs/4, fs/6, fs/8 sines at the stated phases, including +3 dB TP | +0.2/-0.4 dB, all pass |
| 3341 20-23 | an inter-sample peak synthesised at 4 fs and decimated at four offsets | 0.0 +0.2/-0.4 dB TP, all pass |
| 3342 1-4 | loudness range of two- and five-level tone sequences | ±1 LU, all pass |

Cases 7, 8 (3341) and 5, 6 (3342) are authentic programme material and 11, 14 are the live-meter
forms; those need the EBU's files, which `tests/ebu_files.rs` runs when `EBU_TEST_SET_DIR` points at
the unzipped set from [tech.ebu.ch/loudness](https://tech.ebu.ch/loudness). Everything else needs no
download.

Two details the tests forced:

- **Momentary is read every 10 ms, not every 100.** Test 13 places a 400 ms tone at 20 ms offsets and
  expects its maximum within 0.1 LU; on a 100 ms grid the best-aligned window still contains 20 ms of
  silence and reads 0.2 LU low. Integrated gating still steps every 100 ms (the 75 % overlap the
  Recommendation specifies) and the short-term series every 100 ms (Tech 3342 asks for at least 10 Hz).
- **The K-filter is regenerated per sample rate.** BS.1770 tabulates coefficients for 48 kHz only and
  says so; reusing them at 44.1 kHz is the commonest loudness bug. The filter here is built from an
  analog prototype that reproduces the published 48 kHz tables to 1e-12 (a unit test checks) and is
  therefore right at 44.1, 96 and 192 kHz (an integration test checks).

## Layout

```
crates/bs1770/        the library: kweight.rs, meter.rs, lra.rs, truepeak.rs; tests/ hold the EBU cases
crates/lufs/          the CLI (reads WAV: 16/24/32-bit integer and float, any channel count)
crates/bs1770-wasm/   wasm-bindgen bindings: a Meter the page feeds from the Web Audio API
```

## Use it

```bash
cargo install --path crates/lufs            # the CLI
cargo test                                  # the conformance suite
wasm-pack build crates/bs1770-wasm --target web --release   # the browser module (pkg/)
```

```rust
use bs1770::Meter;
let mut meter = Meter::new(48_000.0, 2);
meter.push_interleaved(&samples);           // f32, L R L R ...
let m = meter.finish();
println!("{:.1} LUFS, {:.1} LU, {:.1} dB TP", m.integrated_lufs, m.loudness_range_lu, m.true_peak_dbtp);
```

## Notes on the true-peak detector

The 48-tap polyphase filter is the one tabulated in BS.1770 Annex 2, Attachment 1, used at 4x. The
Recommendation asks for an oversampled rate of at least 192 kHz before a reading is called dB TP, so
below 48 kHz two stages are cascaded (16x). The filter is a detection filter with visible pass-band
ripple (the inner polyphase branches sum to 0.973), which is why the published tolerances are
asymmetric and why a DC step overshoots by nearly a decibel at its edge; the unit tests say so.

## Not done

- Streaming output: the meter keeps every 10 ms sub-block, which is right for files (exact gating) and
  wrong for a live meter that runs for days. A histogram-gated variant is the obvious next step.
- Dialogue gating, LFE weighting and the BS.1771 IIR momentary definition are out of scope by design;
  this is EBU Mode as Tech 3341 defines it.
