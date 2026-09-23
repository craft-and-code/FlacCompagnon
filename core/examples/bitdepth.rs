//! Inspect the same bit-depth measurement used by the app, without running
//! the unrelated, expensive codec-lattice search. All files are read-only.
//!
//! `cargo run --release -p flaccompagnon-core --example bitdepth -- <files...>`

use std::path::Path;

use flaccompagnon_core::{analysis::detections::classify, decode, transcode::LatticeSkip};

fn main() {
    for arg in std::env::args_os().skip(1) {
        let path = Path::new(&arg);
        let decoded = if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("flac"))
        {
            decode::decode_and_analyze_flac(path, false).map(|(out, _)| out)
        } else {
            decode::decode_and_analyze(path)
        };
        match decoded {
            Ok(out) => {
                let summary = out.analyzer.finish(out.sample_rate, out.declared_bits);
                // The codec search is deliberately skipped; classify still
                // applies the app's real, independent upscaling rule.
                let upscaled = classify(
                    &summary,
                    out.sample_rate,
                    out.declared_bits,
                    summary.real_bit_depth,
                    Err(LatticeSkip::Cancelled),
                )
                .upscaling;
                println!(
                    "{}\ndeclared={:?}, real={:?}, evidence={:?}, upscaled={upscaled}",
                    path.display(),
                    out.declared_bits,
                    summary.real_bit_depth,
                    summary.bit_depth_evidence
                );
            }
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
    }
}
