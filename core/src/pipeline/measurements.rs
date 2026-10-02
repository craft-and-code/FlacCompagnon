//! Map decoded measurements and requested authenticity evidence into a record.
use crate::analysis::detections;
use crate::decode;
use crate::dsd as dsd_format;
use crate::types::FileAnalysis;

/// Fold a successful decode into `result`, and return the DSD-heritage rise
/// (in dB) if the ultrasonic content shows one.
pub(super) fn apply_outcome(
    outcome: decode::DecodeOutcome,
    result: &mut FileAnalysis,
    transcoded: Option<crate::transcode::LatticeResult>,
) -> Option<f32> {
    result.format = outcome.format;
    result.codec = outcome.codec;
    result.sample_rate = outcome.sample_rate;
    result.channels = outcome.channels;
    result.declared_bits = outcome.declared_bits;
    result.duration_secs = outcome.duration_secs;

    let summary = outcome
        .analyzer
        .finish(outcome.sample_rate, outcome.declared_bits);

    result.cutoff_hz = Some(summary.cutoff_hz);
    result.cutoff_ratio = Some(summary.cutoff_ratio);
    // The *old* detector's `summary.requant_rate` is no longer surfaced: it
    // came from the spectral machinery this rewrite replaced. What the column
    // shows now is the lattice search's own score.
    // Borrowed, not consumed: `classify` needs the same value below, and
    // `LatticeResult` carries a `String` in its error case so it is not `Copy`.
    result.lattice_score = transcoded
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|e| e.likelihood as f32);
    result.clipping = summary.clipping.clone();
    result.dr_db = summary.dr_db;
    result.integrated_lufs = summary.integrated_lufs;
    result.loudness_peaks = summary.loudness_peaks;
    result.loudness_range_lu = summary.loudness_range_lu;
    // Bounded event lists are cheap to copy; classification still borrows
    // the complete summary below.
    result.discontinuities = summary.discontinuities.clone();
    result.stereo_balance = summary.stereo_balance;
    result.high_frequency_stereo = summary.high_frequency_stereo;
    // At most 32 means; retain the summary for classification below.
    result.dc_offset = summary.dc_offset.clone();
    result.local_phase = summary.local_phase;
    result.bit_depth_evidence = summary.bit_depth_evidence;

    if outcome.channels >= 2 {
        result.fake_stereo = Some(summary.fake_stereo);
    }
    if outcome.channels == 2 {
        result.phase_correlation = summary.phase_correlation;
        result.phase_inverted = summary.phase_correlation.map(|_| summary.phase_inverted);
    }
    let real_bits = match (outcome.declared_bits, summary.real_bit_depth) {
        (Some(_), Some(real)) => {
            result.real_bit_depth = Some(real);
            Some(real)
        }
        _ => None,
    };
    // `transcoded` is handed in — see the call site for why. An `Err` there
    // means "no answer" (unsupported rate, too short, undecodable), never
    // "clean": an un-run test must leave the flag down and say why, which is
    // what carrying the reason all the way here buys.
    let transcoded = transcoded?;
    result.detections = detections::classify(
        &summary,
        outcome.sample_rate,
        outcome.declared_bits,
        real_bits,
        transcoded,
    );

    dsd_format::dsd_heritage_check(
        &summary.spectrum_db,
        outcome.sample_rate,
        summary.fft_size(),
    )
}
