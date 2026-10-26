use super::*;

fn fixture(path: &Path) {
    let frames = 4410_u32;
    let data_len = frames * 2;
    let mut wav = b"RIFF".to_vec();
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&44100_u32.to_le_bytes());
    wav.extend_from_slice(&88200_u32.to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.resize(44 + data_len as usize, 0);
    std::fs::write(path, wav).unwrap();
}

#[test]
fn standard_and_large_dimensions_preserve_the_requested_plot_sizes() {
    assert_eq!(SpectrogramSize::default().dimensions(), (1200, 627));
    assert_eq!(SpectrogramSize::Full.dimensions(), (1800, 940));
    assert_eq!(SpectrogramColor::default(), SpectrogramColor::Green);
}

#[test]
fn native_renderer_writes_a_png_without_modifying_audio() {
    let folder = tempfile::tempdir().unwrap();
    let input = folder.path().join("silent.wav");
    let output = folder.path().join("silent.png");
    fixture(&input);
    let before = std::fs::read(&input).unwrap();
    render(
        &input,
        &output,
        SpectrogramSize::Half,
        SpectrogramColor::Green,
        &|| false,
    )
    .unwrap();
    let bytes = std::fs::read(&output).unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 1448);
    assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 903);
    assert_eq!(std::fs::read(&input).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn a_spectrogram_link_cannot_overwrite_the_original_audio() {
    let folder = tempfile::tempdir().unwrap();
    let input = folder.path().join("track.wav");
    let output = folder.path().join("track.png");
    fixture(&input);
    let before = std::fs::read(&input).unwrap();
    std::os::unix::fs::symlink(&input, &output).unwrap();
    let error = render(
        &input,
        &output,
        SpectrogramSize::Half,
        SpectrogramColor::Green,
        &|| false,
    )
    .unwrap_err();
    assert!(error.contains("symbolic link"));
    assert_eq!(std::fs::read(&input).unwrap(), before);
}

#[test]
fn a_wrong_destination_cannot_replace_the_source_audio() {
    let folder = tempfile::tempdir().unwrap();
    let input = folder.path().join("track.wav");
    fixture(&input);
    assert!(render(
        &input,
        &input,
        SpectrogramSize::Half,
        SpectrogramColor::Green,
        &|| false
    )
    .is_err());
}

#[test]
fn cancellation_and_decode_failure_preserve_an_existing_png() {
    let folder = tempfile::tempdir().unwrap();
    let input = folder.path().join("track.wav");
    let output = folder.path().join("track.png");
    fixture(&input);
    std::fs::write(&output, b"previous PNG").unwrap();
    assert!(render(
        &input,
        &output,
        SpectrogramSize::Half,
        SpectrogramColor::Green,
        &|| true
    )
    .is_err());
    std::fs::write(&input, b"malformed WAV").unwrap();
    assert!(render(
        &input,
        &output,
        SpectrogramSize::Half,
        SpectrogramColor::Green,
        &|| false
    )
    .is_err());
    assert_eq!(std::fs::read(&output).unwrap(), b"previous PNG");
}
