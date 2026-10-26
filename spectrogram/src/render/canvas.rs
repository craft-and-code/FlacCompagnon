//! Clipped RGB drawing primitives shared by text, axes and annotations.

use crate::{Color, Error};

pub(super) struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(width: u32, height: u32, color: Color) -> Result<Self, Error> {
        let length = width as usize * height as usize * 3;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(length)
            .map_err(|e| Error::Render(format!("image allocation failed: {e}")))?;
        for _ in 0..width as usize * height as usize {
            pixels.extend_from_slice(&color);
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
    pub fn pixel(&mut self, x: u32, y: u32, color: Color) {
        if x < self.width && y < self.height {
            let index = (y as usize * self.width as usize + x as usize) * 3;
            if let Some(pixel) = self.pixels.get_mut(index..index + 3) {
                pixel.copy_from_slice(&color);
            }
        }
    }
    pub fn blend(&mut self, x: i32, y: i32, color: Color, alpha: u8) {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return;
        }
        let index = (y as usize * self.width as usize + x as usize) * 3;
        if let Some(pixel) = self.pixels.get_mut(index..index + 3) {
            for (value, ink) in pixel.iter_mut().zip(color) {
                *value = ((u32::from(*value) * (255 - u32::from(alpha))
                    + u32::from(ink) * u32::from(alpha)
                    + 127)
                    / 255) as u8;
            }
        }
    }
    pub fn horizontal(&mut self, x: u32, y: u32, width: u32, color: Color, dashed: bool) {
        for dx in 0..width.min(self.width.saturating_sub(x)) {
            if !dashed || dx % 14 < 7 {
                self.pixel(x + dx, y, color);
            }
        }
    }
    pub fn vertical(&mut self, x: u32, y: u32, height: u32, color: Color, dashed: bool) {
        for dy in 0..height.min(self.height.saturating_sub(y)) {
            if !dashed || dy % 14 < 7 {
                self.pixel(x, y + dy, color);
            }
        }
    }
}
