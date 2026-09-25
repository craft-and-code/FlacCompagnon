//! Run the transcoding detectors on one real file and print what they found.
//!
//! Not a test: it asserts nothing, it needs a file that cannot live in the
//! repository, and it takes seconds to minutes. Keeping it in `tests/` meant
//! it sat in every `cargo test` run as a permanently `ignored` line, which is
//! noise — an example is what Cargo has for a program you run by hand.
//!
//! ```text
//! cargo run --release -p flaccompagnon-core --example probe -- "/path/to/track.flac"
//! ```
//!
//! It prints the raw likelihood, the winning alignment and the verdict for
//! **both** codecs, which is what you want when the app's Detection column
//! says something surprising: it runs the same detector code without going
//! through the Tauri binary, so it separates "the algorithm is wrong" from
//! "the app I am running was built before the algorithm changed".

use std::path::PathBuf;

use flaccompagnon_core::decode::decode_to_pcm;
use flaccompagnon_core::transcode::{aac, mp3, AacParams, Mp3Params};

/// Split interleaved samples into one `f64` buffer per channel, keeping at
/// most two — the same thing `pipeline::transcode` does internally, repeated
/// here because that helper is private and this probe deliberately calls the
/// detectors directly.
fn deinterleave(samples: &[f32], channels: usize) -> Vec<Vec<f64>> {
    if channels == 0 || samples.is_empty() {
        return Vec::new();
    }
    let kept = channels.min(2);
    let frames = samples.len() / channels;
    let mut out = vec![Vec::with_capacity(frames); kept];
    for f in 0..frames {
        for (c, buf) in out.iter_mut().enumerate() {
            if let Some(s) = samples.get(f * channels + c) {
                buf.push(*s as f64);
            }
        }
    }
    out
}

fn main() {
    let Some(arg) = std::env::args().nth(1) else {
        eprintln!(
            "usage: cargo run --release -p flaccompagnon-core --example probe -- <audio file>"
        );
        std::process::exit(2);
    };
    let path = PathBuf::from(arg);

    let pcm = match decode_to_pcm(&path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("could not decode {}: {e}", path.display());
            std::process::exit(1);
        }
    };
    let channels = deinterleave(&pcm.samples, pcm.channels);
    println!(
        "\nfile      : {}\nrate      : {} Hz, {} channel(s), {} frames\n",
        path.display(),
        pcm.sample_rate,
        pcm.channels,
        channels.first().map(|c| c.len()).unwrap_or(0),
    );

    let aac_params = AacParams::default();
    let mp3_params = Mp3Params::default();
    let never = || false;

    // Each channel on its own first. `detect` takes the best of L/R/M/S,
    // which hides *which* signal carried the finding — and if one of them is
    // producing nonsense, the maximum hides that too.
    for (i, ch) in channels.iter().enumerate() {
        let one = vec![ch.clone()];
        match mp3::detect(&one, pcm.sample_rate, &mp3_params, &never) {
            Some(e) => println!(
                "MP3 channel {i}: L = {:.4}  offset = {:4}  -> {}",
                e.likelihood,
                e.offset,
                if e.detected { "TRANSCODED" } else { "clean" }
            ),
            None => println!("MP3 channel {i}: no answer"),
        }
    }
    println!();

    let started = std::time::Instant::now();
    let a = aac::detect(&channels, pcm.sample_rate, &aac_params, &never);
    let aac_secs = started.elapsed().as_secs_f64();

    let started = std::time::Instant::now();
    let m = mp3::detect(&channels, pcm.sample_rate, &mp3_params, &never);
    let mp3_secs = started.elapsed().as_secs_f64();

    let show = |name: &str,
                e: Option<flaccompagnon_core::transcode::TranscodeEvidence>,
                lambda: f64,
                secs: f64| {
        match e {
            Some(e) => println!(
                "{name:4}: L = {:.4}  (λ = {lambda})  offset = {:4}  ->  {}   [{secs:.1} s]",
                e.likelihood,
                e.offset,
                if e.detected { "TRANSCODED" } else { "clean" },
            ),
            // Not "clean": the detector never ran. An untabulated sample rate
            // (anything but 32/44.1/48 kHz) or a file too short lands here.
            None => {
                println!("{name:4}: no answer — unsupported rate or file too short  [{secs:.1} s]")
            }
        }
    };
    show("AAC", a, aac_params.significance, aac_secs);
    show("MP3", m, mp3_params.significance, mp3_secs);
    println!();
}
