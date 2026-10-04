use super::*;

fn info(format: &str) -> BasicInfo {
    BasicInfo {
        sample_rate: 44_100,
        channels: 2,
        bits: Some(16),
        format: format.to_string(),
    }
}

#[test]
fn caption_reads_like_a_caption() {
    let c = caption(&info("FLAC"));
    assert_eq!(c, "44100 Hz | 16-bit | 2 ch | FLAC | Nyquist 22050 Hz");
}

#[test]
fn float_sources_have_no_bit_depth() {
    let mut i = info("WAV");
    i.bits = None;
    assert!(caption(&i).contains("float"));
    assert!(!caption(&i).contains("-bit"));
}

/// Every character that could end a `drawtext` argument, start another
/// filter, or escape out of the expression must be dropped. `format`
/// comes from the file's own extension, so it is attacker-chosen: a file
/// named `x.'a,b` would otherwise inject into the filter graph ffmpeg is
/// handed.
///
/// This is the test the whole `filter()` in `caption` exists for — if
/// someone widens that character set, it fails here rather than in
/// someone's music folder.
#[test]
fn caption_cannot_break_out_of_the_ffmpeg_filter_graph() {
    let hostile = "A'B\"C:D,E\\F;G=H[I]J{K}L`M$N\nO\rP\tQ%R*S?T<U>V&W(X)";
    let c = caption(&info(hostile));
    for bad in [
        '\'', '"', ':', ',', '\\', ';', '=', '[', ']', '{', '}', '`', '$', '\n', '\r', '\t', '%',
        '*', '?', '<', '>', '&', '(', ')',
    ] {
        assert!(!c.contains(bad), "{bad:?} survived in {c:?}");
    }
    // The harmless letters are still there, so the filter isn't just
    // emptying the string.
    assert!(c.contains("ABCDEF"), "{c:?}");
}

/// A format label is derived from an arbitrary file extension, which can
/// be any Unicode at all. Non-ASCII is dropped rather than passed to
/// ffmpeg's font renderer, and nothing panics on a multi-byte character.
#[test]
fn caption_drops_non_ascii_without_panicking() {
    for weird in ["é", "日本語", "🎵", "A\u{202E}B", "\u{0}"] {
        let c = caption(&info(weird));
        assert!(c.is_ascii(), "{c:?}");
        assert!(c.starts_with("44100 Hz"), "{c:?}");
    }
}

/// The caption is built from numbers the container declared, which may be
/// absurd on a malformed file. Nothing here may divide by zero or panic.
#[test]
fn caption_survives_degenerate_stream_parameters() {
    let mut i = info("FLAC");
    i.sample_rate = 0;
    i.channels = 0;
    i.bits = Some(0);
    assert!(caption(&i).contains("Nyquist 0 Hz"));

    let mut i = info("FLAC");
    i.sample_rate = u32::MAX;
    assert!(!caption(&i).is_empty());
}

/// PATH is searched first, so a user who installed ffmpeg anywhere and put
/// it on their PATH wins over the hardcoded list.
#[test]
fn the_bare_name_is_tried_first() {
    assert_eq!(candidates().first().map(String::as_str), Some("ffmpeg"));
}

/// The absolute paths exist precisely because an app launched from Finder
/// (or a .desktop file) does not inherit the shell's PATH. Dropping the
/// package-manager location would silently break the feature for most
/// users while still passing every test run from a terminal.
#[test]
fn the_usual_install_locations_are_covered() {
    let c = candidates();
    #[cfg(target_os = "macos")]
    assert!(c.iter().any(|p| p == "/opt/homebrew/bin/ffmpeg"), "{c:?}");
    #[cfg(target_os = "linux")]
    assert!(c.iter().any(|p| p == "/usr/bin/ffmpeg"), "{c:?}");
    #[cfg(target_os = "windows")]
    assert!(c.iter().any(|p| p.ends_with("ffmpeg.exe")), "{c:?}");
    // No duplicates: each candidate costs a process spawn to probe.
    let mut sorted = c.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), c.len(), "duplicate candidate in {c:?}");
}

#[test]
fn an_explicit_override_wins_over_the_search() {
    let got = resolve_with(Some("/custom/ffmpeg".to_string()), |_| true);
    assert_eq!(got.as_deref(), Some("/custom/ffmpeg"));
}

/// A stale `FLACCOMPAGNON_FFMPEG` left in a shell profile must not disable
/// the feature — the normal search still runs.
#[test]
fn a_broken_override_falls_back_to_the_search() {
    let got = resolve_with(Some("/gone/ffmpeg".to_string()), |p| p != "/gone/ffmpeg");
    assert_eq!(got.as_deref(), Some("ffmpeg"));
    // An empty variable is treated as unset, not as an empty path.
    let got = resolve_with(Some(String::new()), |p| !p.is_empty());
    assert_eq!(got.as_deref(), Some("ffmpeg"));
}

#[test]
fn nothing_found_is_none_not_a_panic() {
    assert_eq!(resolve_with(None, |_| false), None);
    assert_eq!(resolve_with(Some("/x".to_string()), |_| false), None);
}

/// Probing a path that isn't an executable must answer "no", not fail.
#[test]
fn probing_a_nonexistent_binary_is_false() {
    assert!(!ffmpeg_works("/definitely/not/a/binary/anywhere"));
    assert!(!ffmpeg_works(""));
}

#[test]
fn default_and_full_dimensions_match_aede() {
    assert_eq!(SpectrogramSize::default(), SpectrogramSize::Half);
    assert!(spectrum(SpectrogramSize::Half).contains("s=900x470"));
    assert!(spectrum(SpectrogramSize::Full).contains("s=1800x940"));
}

#[cfg(unix)]
#[test]
fn a_spectrogram_link_cannot_overwrite_the_original_audio() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("track.flac");
    let output = dir.path().join("track.png");
    std::fs::write(&input, b"original audio").unwrap();
    std::os::unix::fs::symlink(&input, &output).unwrap();
    let error = render(
        "unused-ffmpeg",
        &input,
        &output,
        None,
        SpectrogramSize::Half,
    )
    .unwrap_err();
    assert!(error.contains("symbolic link"));
    assert_eq!(std::fs::read(input).unwrap(), b"original audio");
}

// The feature deliberately depends on a system ffmpeg, so this integration
// check runs only on machines that can render spectrograms.
#[test]
fn renders_a_png_from_stdout_without_changing_the_audio() {
    let Some(ffmpeg) = resolve_ffmpeg() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("silent.wav");
    let output = dir.path().join("silent.png");
    let data_len = 4410u32 * 2;
    // A standard RIFF PCM header with 0.1s of mono silence, independent of
    // the app's decoders and encoders.
    let mut wav = b"RIFF".to_vec();
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&44_100u32.to_le_bytes());
    wav.extend_from_slice(&88_200u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.resize(44 + data_len as usize, 0);
    std::fs::write(&input, &wav).unwrap();
    std::fs::write(&output, b"previous render").unwrap();
    render(&ffmpeg, &input, &output, None, SpectrogramSize::Half).unwrap();
    assert!(std::fs::read(output)
        .unwrap()
        .starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(std::fs::read(input).unwrap(), wav);
}

#[test]
fn a_wrong_spectrogram_destination_cannot_replace_the_source_audio() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("track.flac");
    std::fs::write(&input, b"original audio").unwrap();
    let error = render("unused-ffmpeg", &input, &input, None, SpectrogramSize::Half).unwrap_err();
    assert!(error.contains(".png extension"));
    assert_eq!(std::fs::read(input).unwrap(), b"original audio");
}
