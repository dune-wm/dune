//! The watermark: the number of the desktop a monitor shows (`3`, `🔒 3` when locked),
//! big and faint in its top-left corner. The compositor draws it, so it takes no clicks,
//! no focus and no room (`[watermark]`).

use std::collections::HashMap;

use dune_common::font::{Font, Style};
use smithay::{
    backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
    utils::Transform,
};

/// Pixels around the text inside its buffer, so the shadow is not cut.
const PADDING: i32 = 4;
/// Pixels from the corner of the area tiles get to the buffer.
pub const INSET: i32 = 12;

/// How the watermark looks (`[watermark]`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WatermarkStyle {
    pub enabled: bool,
    /// Pixels of its text.
    pub size: i32,
    /// How much it shows: 0 not at all, 1 fully.
    pub alpha: f32,
    pub color: [u8; 3],
}

impl Default for WatermarkStyle {
    fn default() -> Self {
        WatermarkStyle { enabled: true, size: 64, alpha: 0.2, color: [0xff, 0xff, 0xff] }
    }
}

/// Rendered watermarks, by their label, redrawn only when a new label shows.
pub struct Watermark {
    pub style: WatermarkStyle,
    family: String,
    /// Loaded on first use: fontconfig runs a process.
    font: Option<Option<Font>>,
    labels: HashMap<String, ((i32, i32), MemoryRenderBuffer)>,
}

impl Watermark {
    pub fn new(family: &str, style: WatermarkStyle) -> Self {
        Watermark { style, family: family.to_string(), font: None, labels: HashMap::new() }
    }

    /// The buffer showing `label`, and its size; none without a font.
    pub fn label(&mut self, label: &str) -> Option<(&MemoryRenderBuffer, (i32, i32))> {
        if !self.labels.contains_key(label) {
            let (family, size, color) = (self.family.clone(), self.style.size as f32, self.style.color);
            let font = self
                .font
                .get_or_insert_with(|| {
                    Font::load(&family, size)
                        .or_else(|_| Font::load("monospace", size))
                        .inspect_err(|err| tracing::warn!("watermark without text: {err}"))
                        .ok()
                })
                .as_mut()?;
            let (pixels, size) = draw_label(font, label, color);
            let buffer = MemoryRenderBuffer::from_slice(&pixels, Fourcc::Argb8888, size, 1, Transform::Normal, None);
            self.labels.insert(label.to_string(), (size, buffer));
        }
        let (size, buffer) = &self.labels[label];
        Some((buffer, *size))
    }
}

/// Premultiplied ARGB8888 pixels of `label` in `color` on nothing, with a dark shadow
/// down and to the right so it reads on light windows too; and their size.
fn draw_label(font: &mut Font, label: &str, color: [u8; 3]) -> (Vec<u8>, (i32, i32)) {
    let cell = font.cell_width as i32;
    let shadow = (font.cell_height as i32 / 24).max(1);
    // Where each glyph starts: a cell on, or more for a wide one (an emoji).
    let mut starts = Vec::new();
    let mut x = PADDING;
    for c in label.chars() {
        starts.push(x);
        let wide = font.glyph(c, Style::Regular).map_or(0, |g| g.left + g.width as i32);
        x += cell.max(wide);
    }
    let (w, h) = (x + PADDING + shadow, font.cell_height as i32 + 2 * PADDING + shadow);
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    let baseline = PADDING + font.baseline;
    for (offset, rgb, strength) in [(shadow, [0, 0, 0], 0.6), (0, color, 1.0)] {
        for (c, start) in label.chars().zip(&starts) {
            let Some(glyph) = font.glyph(c, Style::Regular) else { continue };
            let (left, top) = (start + glyph.left + offset, baseline - glyph.top + offset);
            for gy in 0..glyph.height as i32 {
                for gx in 0..glyph.width as i32 {
                    let (x, y) = (left + gx, top + gy);
                    if x < 0 || y < 0 || x >= w || y >= h {
                        continue;
                    }
                    let g = (gy * glyph.width as i32 + gx) as usize;
                    let (rgb, alpha) = if glyph.color {
                        let p = &glyph.data[g * 4..g * 4 + 4];
                        // The shadow of a color glyph is dark too.
                        (if offset > 0 { rgb } else { [p[0], p[1], p[2]] }, p[3])
                    } else {
                        (rgb, glyph.data[g])
                    };
                    let a = (alpha as f32 * strength) as u32;
                    let at = ((y * w + x) * 4) as usize;
                    // Over what is there, premultiplied: B, G, R, A.
                    for (channel, value) in [(0, rgb[2]), (1, rgb[1]), (2, rgb[0]), (3, 255)] {
                        let under = pixels[at + channel] as u32;
                        pixels[at + channel] = ((value as u32 * a + under * (255 - a)) / 255) as u8;
                    }
                }
            }
        }
    }
    (pixels, (w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_watermark_is_premultiplied_text_on_nothing() {
        let Ok(mut font) = Font::load("monospace", 64.0) else { return };
        let (pixels, (w, h)) = draw_label(&mut font, "3", [0xff, 0xff, 0xff]);
        assert_eq!(pixels.len(), (w * h * 4) as usize);
        // The corners are clear; some pixel is lit; no color goes past its alpha.
        assert_eq!(&pixels[..4], &[0, 0, 0, 0]);
        assert!(pixels.chunks(4).any(|p| p[3] == 255 && p[0] == 255));
        assert!(pixels.chunks(4).all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3]));
    }

    #[test]
    fn a_longer_label_is_wider() {
        let Ok(mut font) = Font::load("monospace", 32.0) else { return };
        let (_, (one, _)) = draw_label(&mut font, "3", [0xff; 3]);
        let (_, (two, _)) = draw_label(&mut font, "13", [0xff; 3]);
        assert!(two > one);
    }
}
