// bmpf2fnt – convert Stranded II .bmpf bitmap fonts to BMFont .fnt
// Copyright (C) 2024  bmpf2fnt contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::bmpf::BmpfFont;
use crate::error::BmpfError;

const MAGENTA_BGR: (u8, u8, u8) = (255, 0, 255);
const ALPHA_TOLERANCE: u8 = 10;

#[derive(Debug, Clone)]
pub struct GlyphRegion {
    pub code: u8,
    pub x_advance: f32,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub x_offset: i32,
    pub y_offset: i32,
}

#[derive(Debug, Clone)]
pub struct FontAtlas {
    pub texture_w: u32,
    pub texture_h: u32,
    pub glyphs: Vec<GlyphRegion>,
    pub line_height: u32,
    pub base: u32,
}

#[derive(Debug, Clone)]
struct GlyphBounds {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

pub fn build_font_atlas(
    bmp_rgba: &[u8],
    img_w: u32,
    img_h: u32,
    bmpf: &BmpfFont,
) -> Result<FontAtlas, BmpfError> {
    let glyphs = find_glyph_regions(bmp_rgba, img_w, img_h)?;

    if glyphs.is_empty() {
        return Err(BmpfError::NoGlyphsFound);
    }

    let mut region_index = 0usize;
    let mut result = Vec::with_capacity(bmpf.chars.len());

    for bc in &bmpf.chars {
        if region_index < glyphs.len()
            && glyph_region_contains(&glyphs[region_index], bmp_rgba, img_w, img_h)
        {
            let gr = &glyphs[region_index];
            result.push(GlyphRegion {
                code: bc.code,
                x_advance: bc.advance as f32,
                x: gr.x,
                y: gr.y,
                w: gr.w,
                h: gr.h,
                x_offset: 0,
                y_offset: 0,
            });
            region_index += 1;
        } else {
            result.push(GlyphRegion {
                code: bc.code,
                x_advance: bc.advance as f32,
                x: 0,
                y: 0,
                w: 0,
                h: 0,
                x_offset: 0,
                y_offset: 0,
            });
        }
    }

    let line_h = bmpf.height.max(1) as u32;
    let base = result
        .iter()
        .filter(|g| g.h > 0)
        .map(|g| g.y + g.h)
        .max()
        .unwrap_or(line_h)
        .min(line_h);

    Ok(FontAtlas {
        texture_w: img_w,
        texture_h: img_h,
        glyphs: result,
        line_height: line_h,
        base,
    })
}

fn find_glyph_regions(rgba: &[u8], w: u32, h: u32) -> Result<Vec<GlyphBounds>, BmpfError> {
    let w = w as usize;
    let h = h as usize;
    if rgba.len() < w * h * 4 {
        return Err(BmpfError::Io("image buffer too small".into()));
    }

    let mut visited = vec![false; w * h];
    let mut regions = Vec::new();

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            if visited[y * w + x] {
                continue;
            }
            let r = rgba[idx];
            let g = rgba[idx + 1];
            let b = rgba[idx + 2];
            let a = rgba[idx + 3];
            if is_pixel_empty(r, g, b, a) {
                continue;
            }

            let mut stack = vec![(x, y)];
            let mut min_x = x;
            let mut max_x = x;
            let mut min_y = y;
            let mut max_y = y;

            while let Some((cx, cy)) = stack.pop() {
                if cx >= w || cy >= h || visited[cy * w + cx] {
                    continue;
                }
                let pi = (cy * w + cx) * 4;
                if is_pixel_empty(rgba[pi], rgba[pi + 1], rgba[pi + 2], rgba[pi + 3]) {
                    continue;
                }
                visited[cy * w + cx] = true;
                min_x = min_x.min(cx);
                max_x = max_x.max(cx);
                min_y = min_y.min(cy);
                max_y = max_y.max(cy);

                if cx > 0 {
                    stack.push((cx - 1, cy));
                }
                if cx + 1 < w {
                    stack.push((cx + 1, cy));
                }
                if cy > 0 {
                    stack.push((cx, cy - 1));
                }
                if cy + 1 < h {
                    stack.push((cx, cy + 1));
                }
            }

            regions.push(GlyphBounds {
                x: min_x as u32,
                y: min_y as u32,
                w: (max_x - min_x + 1) as u32,
                h: (max_y - min_y + 1) as u32,
            });
        }
    }

    regions.sort_by_key(|r| (r.y, r.x));
    Ok(regions)
}

fn glyph_region_contains(gr: &GlyphBounds, rgba: &[u8], img_w: u32, img_h: u32) -> bool {
    let w = img_w as usize;
    for dy in 0..gr.h.min(img_h) {
        for dx in 0..gr.w.min(img_w) {
            let px = (gr.x + dx) as usize;
            let py = (gr.y + dy) as usize;
            if px >= img_w as usize || py >= img_h as usize {
                continue;
            }
            let idx = (py * w + px) * 4;
            if idx + 3 < rgba.len()
                && !is_pixel_empty(rgba[idx], rgba[idx + 1], rgba[idx + 2], rgba[idx + 3])
            {
                return true;
            }
        }
    }
    false
}

fn is_pixel_empty(r: u8, g: u8, b: u8, a: u8) -> bool {
    if a == 0 {
        return true;
    }
    let dr = r.abs_diff(MAGENTA_BGR.2);
    let dg = g.abs_diff(MAGENTA_BGR.1);
    let db = b.abs_diff(MAGENTA_BGR.0);
    dr <= ALPHA_TOLERANCE && dg <= ALPHA_TOLERANCE && db <= ALPHA_TOLERANCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_atlas_simple() {
        let bmpf = BmpfFont {
            chars: vec![
                crate::bmpf::BmpfChar {
                    code: 65,
                    advance: 10,
                },
            ],
            height: 16,
        };

        let w = 20u32;
        let h = 20u32;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        // Draw a non-magenta, non-transparent pixel for 'A'
        let px = (5 * w as usize + 5) * 4;
        rgba[px] = 0;
        rgba[px + 1] = 0;
        rgba[px + 2] = 0;
        rgba[px + 3] = 255;

        let atlas = build_font_atlas(&rgba, w, h, &bmpf).unwrap();
        assert_eq!(atlas.glyphs.len(), 1);
        assert_eq!(atlas.glyphs[0].code, 65);
        assert_eq!(atlas.glyphs[0].x, 5);
        assert_eq!(atlas.glyphs[0].y, 5);
    }
}
