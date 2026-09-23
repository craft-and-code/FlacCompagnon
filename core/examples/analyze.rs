//! Read-only end-to-end diagnosis, using the same pipeline as the desktop app.
//!
//! `cargo run --release -p flaccompagnon-core --example analyze -- <files...>`

use std::path::Path;

use flaccompagnon_core::{analyze_file, ScanOptions};

fn main() {
    for arg in std::env::args_os().skip(1) {
        let result = analyze_file(Path::new(&arg), &ScanOptions::default());
        println!(
            "{}\nrate={}, declared={:?}, real={:?}, status={}, flags=({},{},{}), error={:?}\n{}",
            result.file_name,
            result.sample_rate,
            result.declared_bits,
            result.real_bit_depth,
            result.detections.summary,
            result.detections.upscaling,
            result.detections.upsampling,
            result.detections.transcoding,
            result.error,
            result.detections.detail
        );
    }
}
