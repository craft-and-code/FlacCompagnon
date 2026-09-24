//! Flat numeric CSV evidence for local and band phase; JSON keeps the typed shape.

use crate::analysis::local_phase::{LocalPhase, PhaseSummary};

const RANGES: [&str; 5] = [
    "local_phase",
    "phase_20_200",
    "phase_200_2000",
    "phase_2000_6000",
    "phase_6000_20000",
];
const METRICS: [&str; 5] = [
    "correlation",
    "minimum",
    "minimum_start_s",
    "opposed_fraction",
    "eligible_windows",
];

pub(super) fn headers() -> Vec<String> {
    let mut columns: Vec<_> = ["phase_window_s", "phase_hop_s", "phase_analyzed_windows"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    for range in RANGES {
        columns.extend(METRICS.map(|metric| format!("{range}_{metric}")));
    }
    columns
}

pub(super) fn values(phase: Option<&LocalPhase>) -> Vec<String> {
    let Some(phase) = phase else {
        return vec![String::new(); 3 + RANGES.len() * METRICS.len()];
    };
    let mut columns = vec![
        phase.window_secs.to_string(),
        phase.hop_secs.to_string(),
        phase.analyzed_windows.to_string(),
    ];
    for summary in
        std::iter::once(phase.broadband).chain(phase.bands.iter().map(|band| band.summary))
    {
        columns.extend(summary_values(summary));
    }
    columns
}

fn summary_values(summary: Option<PhaseSummary>) -> [String; 5] {
    summary
        .map(|s| {
            [
                s.correlation.to_string(),
                s.minimum_correlation.to_string(),
                s.minimum_start_secs.to_string(),
                s.opposed_fraction.to_string(),
                s.eligible_windows.to_string(),
            ]
        })
        .unwrap_or_default()
}
