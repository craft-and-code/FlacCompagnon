//! Physical axes, legends and caller-provided labels share one layout.

use super::{canvas::Canvas, text, RenderOptions};
use crate::{FrequencyScale, Spectrogram};
use fontdue::Font;

pub(super) fn draw(image: &mut Canvas, font: &Font, data: &Spectrogram, options: &RenderOptions) {
    let [left, top, ..] = options.margins;
    let (width, height) = data.dimensions();
    let right = left + width;
    let bottom = top + height;
    let size = options.font_size;
    let scale = size / 15.0;
    let theme = &options.theme;
    let write = |image: &mut Canvas, label: &str, x: f32, y: f32, px: f32, color, limit| {
        text::draw(image, font, label, (x, y), px, color, limit)
    };
    image.horizontal(left, top.saturating_sub(1), width, theme.border, false);
    image.horizontal(left, bottom, width, theme.border, false);
    image.vertical(left.saturating_sub(1), top, height, theme.border, false);
    image.vertical(right, top, height, theme.border, false);
    if options.show_axes {
        let min = data.options.min_frequency;
        let max = data.max_frequency;
        let mut ticks = vec![min, max];
        match data.options.frequency_scale {
            FrequencyScale::Linear => {
                let step = nice_step((max - min) / 6.0);
                let mut hz = (min / step).ceil() * step;
                while hz < max {
                    ticks.push(hz);
                    hz += step;
                }
            }
            FrequencyScale::Logarithmic => {
                for power in min.log10().floor() as i32..=max.log10().ceil() as i32 {
                    for factor in [1.0, 2.0, 5.0] {
                        let hz = factor * 10.0_f64.powi(power);
                        if hz > min && hz < max {
                            ticks.push(hz);
                        }
                    }
                }
            }
        }
        ticks.sort_by(f64::total_cmp);
        ticks.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        let mut last_y = f64::INFINITY;
        for hz in ticks {
            let y = f64::from(bottom) - data.fraction(hz) * f64::from(height);
            if (last_y - y).abs() < f64::from(size) * 1.6 && hz != max {
                continue;
            }
            let label = compact(hz / 1000.0);
            let label_width = text::width(font, &label, size * 0.8);
            write(
                image,
                &label,
                left as f32 - 14.0 * scale - label_width,
                y as f32 - size * 0.55,
                size * 0.8,
                theme.muted,
                left as f32 - 14.0 * scale,
            );
            image.horizontal(
                left.saturating_sub((6.0 * scale) as u32),
                y.max(0.0) as u32,
                (6.0 * scale) as u32,
                theme.muted,
                false,
            );
            if options.show_grid && hz > min && hz < max {
                image.horizontal(left, y as u32, width, theme.border, true);
            }
            last_y = y;
        }
        write(
            image,
            "kHz",
            left as f32 - 38.0 * scale,
            top as f32 - 24.0 * scale,
            size * 0.8,
            theme.muted,
            38.0 * scale,
        );
        let duration = data.duration();
        let step = nice_step(duration / 6.0);
        let mut ticks = vec![0.0];
        let mut time = step;
        while time < duration - step * 0.4 {
            ticks.push(time);
            time += step;
        }
        ticks.push(duration);
        for time in ticks {
            let x = left as f32 + (time / duration) as f32 * width as f32;
            let label = if duration < 10.0 {
                format!("{time:.2}s")
            } else {
                let seconds = time.round() as u64;
                format!("{}:{:02}", seconds / 60, seconds % 60)
            };
            let label_width = text::width(font, &label, size * 0.8);
            let x = if time == 0.0 {
                x
            } else if time == duration {
                x - label_width
            } else {
                x - label_width / 2.0
            };
            write(
                image,
                &label,
                x,
                bottom as f32 + 10.0 * scale,
                size * 0.8,
                theme.muted,
                label_width + 2.0,
            );
        }
        write(
            image,
            if duration < 10.0 {
                "Time (s)"
            } else {
                "Time (min:s)"
            },
            left as f32 + width as f32 / 2.0 - 48.0 * scale,
            bottom as f32 + 36.0 * scale,
            size * 0.75,
            theme.muted,
            110.0 * scale,
        );
    }
    if options.show_legend {
        let x = right + (24.0 * scale) as u32;
        let bar_width = (18.0 * scale) as u32;
        for row in 0..height {
            let fraction = 1.0 - f64::from(row) / f64::from(height.saturating_sub(1).max(1));
            image.horizontal(
                x,
                top + row,
                bar_width,
                options.palette.color(fraction),
                false,
            );
        }
        write(
            image,
            "dBFS / bin",
            x as f32,
            top as f32 - 24.0 * scale,
            size * 0.75,
            theme.muted,
            100.0 * scale,
        );
        for tick in 0..=6 {
            let t = f64::from(tick) / 6.0;
            let db = options.max_db - t * (options.max_db - options.min_db);
            write(
                image,
                &compact(db),
                (x + bar_width) as f32 + 10.0 * scale,
                top as f32 + (t * f64::from(height - 1)) as f32 - size * 0.55,
                size * 0.75,
                theme.muted,
                70.0 * scale,
            );
        }
    }
}

fn nice_step(target: f64) -> f64 {
    let magnitude = 10.0_f64.powf(target.max(1e-12).log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .find(|&factor| factor * magnitude >= target)
        .unwrap_or(10.0)
        * magnitude
}

fn compact(number: f64) -> String {
    let formatted = format!("{number:.2}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}
