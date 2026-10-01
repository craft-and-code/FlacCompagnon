//! Terminal-width-aware feedback, with plain log lines for redirected output.

use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;
use std::path::Path;
use std::time::Duration;

const LOADING_TEMPLATE: &str = "  {spinner:2.cyan} {elapsed_precise} {wide_msg}";
const ANALYSIS_TEMPLATE: &str =
    "  [{pos}/{len}] {percent}% {wide_bar:.cyan/blue} {elapsed_precise}\n  {spinner:2.cyan} {wide_msg}";

struct Loading(ProgressBar);

impl Drop for Loading {
    fn drop(&mut self) {
        // Clear and stop the animation before results are printed, also on unwind.
        self.0.finish_and_clear();
    }
}

fn interactive() -> bool {
    std::io::stderr().is_terminal() && std::env::var("TERM").map_or(true, |term| term != "dumb")
}

fn style(template: &str) -> ProgressStyle {
    ProgressStyle::with_template(template)
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
        .tick_strings(&["◐", "◓", "◑", "◒", "●"])
        .progress_chars("━╸─")
}

fn with_feedback<T>(bar: ProgressBar, message: String, operation: impl FnOnce() -> T) -> T {
    bar.set_message(message);
    let loading = Loading(bar);
    loading.0.enable_steady_tick(Duration::from_millis(100));
    operation()
}

pub(crate) fn with_loading<T>(message: &str, operation: impl FnOnce() -> T) -> T {
    eprintln!("  ▸ {message}");
    if !interactive() {
        return operation();
    }
    let bar = ProgressBar::new_spinner().with_style(style(LOADING_TEMPLATE));
    with_feedback(bar, message.to_string(), operation)
}

pub(crate) fn with_analysis<T>(
    path: &Path,
    completed: usize,
    total: usize,
    operation: impl FnOnce() -> T,
) -> T {
    // Keep the full path once in the log. Only the filename is animated so a
    // deeply nested album cannot flood terminal scrollback at every tick.
    eprintln!(
        "  ▸ [{}/{}] Analyzing {}",
        completed + 1,
        total,
        path.display()
    );
    if !interactive() {
        return operation();
    }
    let bar = ProgressBar::new(total as u64).with_style(style(ANALYSIS_TEMPLATE));
    bar.set_position(completed as u64);
    let filename = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy();
    with_feedback(bar, format!("Analyzing {filename}"), operation)
}

#[cfg(test)]
#[path = "../tests/unit/progress.rs"]
mod tests;
