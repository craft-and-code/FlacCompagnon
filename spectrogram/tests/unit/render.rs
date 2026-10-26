use super::*;
use crate::{AnalysisOptions, ColorStop, SignalInfo, SpectrogramBuilder};

fn data() -> Spectrogram {
    let mut builder = SpectrogramBuilder::new(
        SignalInfo {
            sample_rate: 48000,
            channels: 1,
            frames: 4096,
        },
        AnalysisOptions {
            width: 320,
            height: 170,
            ..Default::default()
        },
    )
    .unwrap();
    builder.push_interleaved(&vec![0.25; 4096]).unwrap();
    builder.finish().unwrap()
}

#[test]
fn png_round_trip_preserves_exact_rgb_pixels_and_unicode_labels() {
    let data = data();
    let labels = ImageLabels {
        generator: "Aède".into(),
        title: "Été — l’oiseau".into(),
        ..Default::default()
    };
    let options = RenderOptions::default();
    let bytes = render_png(&data, &labels, &options).unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    assert_eq!((decoder.info().width, decoder.info().height), (568, 446));
    let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
    let frame = decoder.next_frame(&mut pixels).unwrap();
    assert_eq!(frame.color_type, png::ColorType::Rgb);
    let expected = options.palette.color(
        (data.level_db(160, 85).unwrap() - options.min_db) / (options.max_db - options.min_db),
    );
    let offset = ((108 + 170 - 1 - 85) * 568 + 104 + 160) * 3;
    assert_eq!(&pixels[offset..offset + 3], &expected);
    assert!(decoder
        .info()
        .utf8_text
        .iter()
        .any(|entry| entry.keyword == "Title" && entry.get_text().unwrap() == labels.title));
}

#[test]
fn palette_changes_only_rgb_not_the_stored_measurements() {
    let data = data();
    let original = data.level_db(100, 0);
    let green = render_png(&data, &ImageLabels::default(), &RenderOptions::default()).unwrap();
    let blue = render_png(
        &data,
        &ImageLabels::default(),
        &RenderOptions {
            palette: Palette::blue(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_ne!(green, blue);
    assert_eq!(original, data.level_db(100, 0));
}

#[test]
fn custom_ramps_and_explicit_level_ranges_are_supported() {
    let data = data();
    let ramp = Palette {
        stops: vec![
            ColorStop {
                position: 0.0,
                color: [0, 0, 0],
            },
            ColorStop {
                position: 0.25,
                color: [0, 30, 60],
            },
            ColorStop {
                position: 1.0,
                color: [255, 255, 255],
            },
        ],
    };
    assert!(render_png(
        &data,
        &ImageLabels::default(),
        &RenderOptions {
            palette: ramp,
            min_db: -144.0,
            max_db: 6.0,
            ..Default::default()
        }
    )
    .is_ok());
}

#[test]
fn malformed_render_parameters_and_fonts_are_errors() {
    let data = data();
    for options in [
        RenderOptions {
            min_db: 0.0,
            max_db: 0.0,
            ..Default::default()
        },
        RenderOptions {
            max_db: f64::NAN,
            ..Default::default()
        },
        RenderOptions {
            font_size: f32::INFINITY,
            ..Default::default()
        },
        RenderOptions {
            palette: Palette { stops: vec![] },
            ..Default::default()
        },
        RenderOptions {
            font: Some(Arc::from(&b"bad font"[..])),
            ..Default::default()
        },
        RenderOptions {
            margins: [u32::MAX; 4],
            ..Default::default()
        },
    ] {
        assert!(render_png(&data, &ImageLabels::default(), &options).is_err());
    }
}

#[test]
fn markers_are_explicit_physical_values_and_outside_values_are_errors() {
    let data = data();
    let mut options = RenderOptions {
        markers: vec![Marker {
            axis: MarkerAxis::Frequency,
            value: 16000.0,
            label: "Measured cutoff".into(),
            color: [241, 240, 235],
            dashed: true,
        }],
        ..Default::default()
    };
    assert!(render_png(&data, &ImageLabels::default(), &options).is_ok());
    options.markers[0].value = 24001.0;
    assert!(render_png(&data, &ImageLabels::default(), &options).is_err());
}
