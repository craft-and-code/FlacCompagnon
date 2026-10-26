//! Continuous, caller-defined color ramps. Colors encode level, not verdicts.

use crate::Error;

/// An opaque sRGB color, in red/green/blue order.
pub type Color = [u8; 3];

/// One color at a normalized position between the lower and upper dB bounds.
#[derive(Debug, Clone)]
pub struct ColorStop {
    /// Position from zero (quiet) to one (loud).
    pub position: f64,
    /// sRGB color at this position.
    pub color: Color,
}

/// A continuous ramp with any number of ordered stops, including both endpoints.
#[derive(Debug, Clone)]
pub struct Palette {
    /// Ordered control points; at least two, at most 256.
    pub stops: Vec<ColorStop>,
}

impl Palette {
    /// Monochrome Phémios-inspired green, shared by both applications by default.
    pub fn green() -> Self {
        let colors = [
            [17, 18, 19],
            [27, 32, 27],
            [55, 69, 38],
            [106, 130, 52],
            [216, 243, 106],
            [241, 240, 235],
        ];
        Self {
            stops: colors
                .into_iter()
                .enumerate()
                .map(|(index, color)| ColorStop {
                    position: index as f64 / 5.0,
                    color,
                })
                .collect(),
        }
    }
    /// Monochrome blue.
    pub fn blue() -> Self {
        Self::monochrome([112, 185, 247])
    }
    /// Monochrome amber.
    pub fn amber() -> Self {
        Self::monochrome([246, 196, 105])
    }
    /// Monochrome violet.
    pub fn violet() -> Self {
        Self::monochrome([196, 163, 247])
    }
    /// Make a dark-to-light ramp around an arbitrary accent.
    pub fn monochrome(accent: Color) -> Self {
        let dark = [17, 18, 19];
        let stops = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0]
            .into_iter()
            .map(|position| {
                let color = if position == 1.0 {
                    [241, 240, 235]
                } else {
                    let amount = (position / 0.8_f64).powf(1.8);
                    std::array::from_fn(|i| {
                        (f64::from(dark[i]) * (1.0 - amount) + f64::from(accent[i]) * amount)
                            .round() as u8
                    })
                };
                ColorStop { position, color }
            })
            .collect();
        Self { stops }
    }
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if !(2..=256).contains(&self.stops.len())
            || self.stops.first().is_none_or(|s| s.position != 0.0)
            || self.stops.last().is_none_or(|s| s.position != 1.0)
            || self.stops.iter().any(|s| !s.position.is_finite())
            || self
                .stops
                .windows(2)
                .any(|s| s[0].position >= s[1].position)
        {
            return Err(Error::Configuration(
                "palette stops must increase from zero to one".into(),
            ));
        }
        Ok(())
    }
    pub(crate) fn color(&self, position: f64) -> Color {
        let position = position.clamp(0.0, 1.0);
        for pair in self.stops.windows(2) {
            if position <= pair[1].position {
                let t = (position - pair[0].position) / (pair[1].position - pair[0].position);
                return std::array::from_fn(|i| {
                    (f64::from(pair[0].color[i]) * (1.0 - t) + f64::from(pair[1].color[i]) * t)
                        .round() as u8
                });
            }
        }
        [241, 240, 235]
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::green()
    }
}
