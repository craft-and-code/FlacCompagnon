use super::*;

#[test]
fn progress_templates_use_terminal_width_for_long_filenames() {
    for template in [LOADING_TEMPLATE, ANALYSIS_TEMPLATE] {
        assert!(ProgressStyle::with_template(template).is_ok());
        assert!(template.contains("{wide_msg}"));
    }
}

#[test]
fn animation_is_finished_before_the_caller_receives_the_result() {
    let bar = ProgressBar::hidden();
    let observer = bar.clone();
    let result = with_feedback(bar, "03 Goodbye to Romance.flac".into(), || 42);
    assert_eq!(result, 42);
    assert!(observer.is_finished());
}

#[test]
fn animation_is_finished_if_analysis_panics() {
    let bar = ProgressBar::hidden();
    let observer = bar.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        with_feedback(bar, "file.flac".into(), || panic!("analysis panic"))
    }));
    assert!(result.is_err());
    assert!(observer.is_finished());
}

#[test]
fn long_unicode_filenames_do_not_accumulate_when_animation_redraws() {
    use indicatif::{InMemoryTerm, ProgressDrawTarget};

    for width in [24, 40, 80] {
        let terminal = InMemoryTerm::new(20, width);
        let bar = ProgressBar::with_draw_target(
            Some(149),
            ProgressDrawTarget::term_like(Box::new(terminal.clone())),
        );
        bar.set_style(style(ANALYSIS_TEMPLATE));
        bar.set_message(format!(
            "Analyzing {}.flac",
            "é音 Goodbye to Romance ".repeat(12)
        ));
        bar.set_position(2);
        for _ in 0..40 {
            bar.tick();
            bar.force_draw();
        }
        let visible = terminal.contents();
        assert!(visible.lines().count() <= 3, "{width} columns: {visible}");
        assert!(visible.contains("2/149"));
        bar.finish_and_clear();
        assert!(
            terminal.contents().trim().is_empty(),
            "old animation remained on screen"
        );
    }
}

#[test]
fn spinner_message_stays_in_the_same_column_across_complete_rotations() {
    use indicatif::{InMemoryTerm, ProgressDrawTarget};
    let terminal = InMemoryTerm::new(5, 80);
    let bar = ProgressBar::with_draw_target(
        None,
        ProgressDrawTarget::term_like(Box::new(terminal.clone())),
    );
    bar.set_style(style(LOADING_TEMPLATE));
    bar.set_message("Analyzing track.flac");
    let mut columns = Vec::new();
    for _ in 0..20 {
        bar.tick();
        bar.force_draw();
        let contents = terminal.contents();
        columns.push(contents.find("Analyzing").expect("visible message"));
    }
    assert!(columns.iter().all(|column| *column == columns[0]));
    bar.finish_and_clear();
}
