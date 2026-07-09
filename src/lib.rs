//! Parse `.bmpf` bitmap font files (Stranded II / Blitz3D format) and
//! generate `.fnt` (BMFont / AngelCode text format) output.
//!
//! # `.bmpf` format
//!
//! ```text
//! [46-byte ASCII header]  "Unreal Software Bitmap Font Wizard bmpf File\r\n"
//! [optional 6-byte meta]   only present in full (256-char) fonts:
//!   u16 LE: char_count     (256)
//!   u16 LE: font_height    (pixels)
//!   u16 LE: unknown
//! [3-byte records …]
//!   u8:    character code
//!   u16 LE: advance width
//! [terminator]             00 00 00
//! ```

use std::fmt;

// ── Public types ───────────────────────────────────────────

/// A parsed `.bmpf` font: character-code → advance-width mapping.
#[derive(Debug, Clone)]
pub struct BmpfFont {
    /// Characters in the order they appear in the file (also image order).
    pub chars: Vec<BmpfChar>,
    /// Rendered height of the font (0 if unknown).
    pub height: u16,
}

#[derive(Debug, Clone)]
pub struct BmpfChar {
    pub code: u8,
    pub advance: u16,
}

/// A single glyph's bounding box within the font texture.
#[derive(Debug, Clone)]
pub struct GlyphRegion {
    pub code: u8,
    /// Advance width from .bmpf – how far to move the cursor.
    pub x_advance: f32,
    /// Pixel bounds within the font-atlas texture.
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    /// Rendering offset relative to the cursor position.
    pub x_offset: i32,
    pub y_offset: i32,
}

/// Complete font-atlas ready for `.fnt` generation.
#[derive(Debug, Clone)]
pub struct FontAtlas {
    /// Width / height of the texture that holds all glyphs.
    pub texture_w: u32,
    pub texture_h: u32,
    /// Per-glyph measurements.
    pub glyphs: Vec<GlyphRegion>,
    /// Line height (font height) in pixels.
    pub line_height: u32,
    /// Baseline offset from top of the cell.
    pub base: u32,
}

// ── Error ──────────────────────────────────────────────────

#[derive(Debug)]
pub enum BmpfError {
    BadHeader,
    BadMagic,
    InvalidRecord { index: usize, message: String },
    NoGlyphsFound,
    Mismatch { bmpf_chars: usize, glyphs_found: usize },
    Image(image::ImageError),
    Io(String),
}

// Cannot derive Clone because image::ImageError is not Clone.
// Manual impl for convenience:
impl Clone for BmpfError {
    fn clone(&self) -> Self {
        match self {
            Self::BadHeader => Self::BadHeader,
            Self::BadMagic => Self::BadMagic,
            Self::InvalidRecord { index, message } => Self::InvalidRecord { index: *index, message: message.clone() },
            Self::NoGlyphsFound => Self::NoGlyphsFound,
            Self::Mismatch { bmpf_chars, glyphs_found } => Self::Mismatch { bmpf_chars: *bmpf_chars, glyphs_found: *glyphs_found },
            Self::Image(_) => Self::Io("image error (not cloneable)".into()),
            Self::Io(msg) => Self::Io(msg.clone()),
        }
    }
}

impl fmt::Display for BmpfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadHeader => write!(f, "file too short for header"),
            Self::BadMagic => write!(f, "not a .bmpf file: bad magic header"),
            Self::InvalidRecord { index, message } => {
                write!(f, "invalid record #{index}: {message}")
            }
            Self::NoGlyphsFound => write!(f, "no glyphs found in image"),
            Self::Mismatch { bmpf_chars, glyphs_found } => {
                write!(f, "bmpf has {bmpf_chars} chars but found {glyphs_found} glyphs")
            }
            Self::Image(e) => write!(f, "image error: {e}"),
            Self::Io(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for BmpfError {}

impl From<image::ImageError> for BmpfError {
    fn from(e: image::ImageError) -> Self {
        Self::Image(e)
    }
}

// ── Magic header bytes ─────────────────────────────────────

const MAGIC: &[u8] = b"Unreal Software Bitmap Font Wizard bmpf File\r\n";
const MAGIC_LEN: usize = 46;

// ── Parsing .bmpf ──────────────────────────────────────────

impl BmpfFont {
    /// Parse a `.bmpf` byte slice into a `BmpfFont`.
    pub fn parse(data: &[u8]) -> Result<Self, BmpfError> {
        if data.len() < MAGIC_LEN {
            return Err(BmpfError::BadHeader);
        }
        if &data[..MAGIC_LEN] != MAGIC {
            return Err(BmpfError::BadMagic);
        }

        let rest = &data[MAGIC_LEN..];

        // Heuristic: if the first 6 bytes look like 256 + reasonable height,
        // treat them as metadata.  We check that byte[0]==0x00 && byte[1]==0x01
        // (256 in LE) or other clues.
        let (meta_size, height) = if rest.len() >= 6
            && rest[0] == 0x00
            && rest[1] == 0x01
        {
            let _cnt = u16::from_le_bytes([rest[0], rest[1]]);
            let h = u16::from_le_bytes([rest[2], rest[3]]);
            (6usize, h)
        } else {
            // No metadata — we must guess the char count from file size.
            // 312 bytes data / 3 = 104 entries (incl. terminator) → 103 chars.
            // For safety, we just parse until terminator without a count.
            (0usize, 0u16)
        };

        let records_data = &rest[meta_size..];
        let mut chars = Vec::new();
        let mut i = 0;

        while i + 3 <= records_data.len() {
            if records_data[i] == 0 && records_data[i + 1] == 0 && records_data[i + 2] == 0 {
                break; // terminator
            }
            let code = records_data[i];
            let advance = u16::from_le_bytes([records_data[i + 1], records_data[i + 2]]);
            chars.push(BmpfChar { code, advance });
            i += 3;
        }

        Ok(BmpfFont { chars, height })
    }
}

// ── Glyph scanning ─────────────────────────────────────────

/// Threshold for considering a pixel "non-transparent".
/// BMP 24-bit has no alpha; magenta (255,0,255) is the Blitz3D colour key.
const MAGENTA_BGR: (u8, u8, u8) = (255, 0, 255); // B=255, G=0, R=255
const ALPHA_TOLERANCE: u8 = 10; // allow slight colour differences

/// Build a `FontAtlas` by scanning the BMP image for glyphs and matching them
/// to `.bmpf` entries.
///
/// `bmp_rgba` – decoded image pixels (RGBA, top-left-first).
/// `bmpf` – the parsed `.bmpf` data.
pub fn build_font_atlas(
    bmp_rgba: &[u8],
    img_w: u32,
    img_h: u32,
    bmpf: &BmpfFont,
) -> Result<FontAtlas, BmpfError> {
    // We work in RGBA space. Convert transparent BGR pixels to (0,0,0,0)
    // so the atlas is clean PNG.
    // First, detect glyph bounding boxes.

    let glyphs = find_glyph_regions(bmp_rgba, img_w, img_h)?;

    if glyphs.is_empty() {
        return Err(BmpfError::NoGlyphsFound);
    }

    // Match glyphs to bmpf chars.
    // Strategy: iterate bmpf chars left-to-right, match with glyphs in order.
    // Empty chars (control codes, space) have no visible glyph — we still emit
    // them with w=0, h=0 and the advance from bmpf.
    let mut region_index = 0usize;
    let mut result = Vec::with_capacity(bmpf.chars.len());

    for bc in &bmpf.chars {
        // Does this code have a visible glyph?
        if region_index < glyphs.len() && glyph_region_contains(&glyphs[region_index], bmp_rgba, img_w, img_h) {
            let gr = &glyphs[region_index];
            result.push(GlyphRegion {
                code: bc.code,
                x_advance: bc.advance as f32,
                x: gr.x,
                y: gr.y,
                w: gr.w,
                h: gr.h,
                x_offset: 0,  // Blitz3D fonts typically have no side-bearing offset
                y_offset: 0,  // will be computed after we know the baseline
            });
            region_index += 1;
        } else {
            // Invisible glyph (space, control char, etc.)
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

    // Determine line-height and baseline from the first few visible glyphs.
    let line_h = bmpf.height.max(1) as u32;
    // Baseline: for Blitz3D bitmap fonts the glyph sits at the bottom of
    // the cell.  We use `y + h` as the baseline from the top.
    let base = result
        .iter()
        .filter(|g| g.h > 0)
        .map(|g| g.y + g.h)
        .max()
        .unwrap_or(line_h);

    Ok(FontAtlas {
        texture_w: img_w,
        texture_h: img_h,
        glyphs: result,
        line_height: line_h,
        base,
    })
}

/// Find all contiguous non-transparent regions in the image.
/// Returns bounding boxes sorted left-to-right, top-to-bottom.
fn find_glyph_regions(
    rgba: &[u8],
    w: u32,
    h: u32,
) -> Result<Vec<GlyphBounds>, BmpfError> {
    let w = w as usize;
    let h = h as usize;
    if rgba.len() < w * h * 4 {
        return Err(BmpfError::Io("image buffer too small".into()));
    }

    // Mark visited pixels
    let mut visited = vec![false; w * h];
    let mut regions = Vec::new();

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            if visited[y * w + x] {
                continue;
            }
            // Check if pixel is non-transparent (has colour)
            let r = rgba[idx];
            let g = rgba[idx + 1];
            let b = rgba[idx + 2];
            let a = rgba[idx + 3];
            if is_pixel_empty(r, g, b, a) {
                continue;
            }

            // Flood-fill to find connected non-transparent region.
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

                // 4-connected neighbours
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

    // Sort left-to-right, then top-to-bottom.
    regions.sort_by_key(|r| (r.y, r.x));

    Ok(regions)
}

#[derive(Debug, Clone)]
struct GlyphBounds {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

/// Check if a glyph region actually contains non-transparent pixels.
/// (Used to distinguish spacer runs from actual glyphs.)
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

/// A pixel is "empty" if it's fully transparent (alpha=0) OR
/// matches the magenta colour key.
fn is_pixel_empty(r: u8, g: u8, b: u8, a: u8) -> bool {
    if a == 0 {
        return true;
    }
    // Magenta colour key (after conversion we may have α=255, so check colour)
    let dr = r.abs_diff(MAGENTA_BGR.2);
    let dg = g.abs_diff(MAGENTA_BGR.1);
    let db = b.abs_diff(MAGENTA_BGR.0);
    dr <= ALPHA_TOLERANCE && dg <= ALPHA_TOLERANCE && db <= ALPHA_TOLERANCE
}

// ── BMFont .fnt generation ────────────────────────────────

/// Generate BMFont (AngelCode) `.fnt` text format content from a `FontAtlas`.
pub fn generate_bmfont(font: &FontAtlas, name: &str, texture_rel_path: &str) -> String {
    use std::fmt::Write;

    let mut out = String::new();

    // Info line
    let _ = writeln!(out,
        r#"info face="{}" size={} bold=0 italic=0 charset="" unicode=0 stretchH=100 smooth=1 aa=1 padding=0,0,0,0 spacing=0,0"#,
        name, font.line_height,
    );

    // Common line
    let _ = writeln!(out,
        "common lineHeight={} base={} scaleW={} scaleH={} pages=1 packed=0",
        font.line_height, font.base, font.texture_w, font.texture_h,
    );

    // Page line
    let _ = writeln!(out, r#"page id=0 file="{}""#, texture_rel_path);

    // Chars header
    let visible: Vec<&GlyphRegion> = font.glyphs.iter().collect();
    let _ = writeln!(out, "chars count={}", visible.len());

    // Each char
    for g in &visible {
        let _ = writeln!(
            out,
            "char id={} x={} y={} w={} h={} xoffset={} yoffset={} xadvance={} page=0 chnl=0",
            g.code as u32,
            g.x,
            g.y,
            g.w,
            g.h,
            g.x_offset,
            g.y_offset,
            g.x_advance as u32,
        );
    }

    out
}

// ── Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Hex string → Vec<u8>
    #[allow(dead_code)]
    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn test_parse_font_norm() {
        // Build a minimal .bmpf with header + 3 characters + terminator
        let mut data = MAGIC.to_vec();
        // char 'e'(0x65) advance=16, char '!' advance=6, char '"' advance=9
        data.extend_from_slice(&[0x65, 0x10, 0x00]); // e
        data.extend_from_slice(&[0x21, 0x06, 0x00]); // !
        data.extend_from_slice(&[0x22, 0x09, 0x00]); // "
        data.extend_from_slice(&[0x00, 0x00, 0x00]); // terminator

        let font = BmpfFont::parse(&data).unwrap();
        assert_eq!(font.chars.len(), 3);
        assert_eq!(font.chars[0].code, 0x65);
        assert_eq!(font.chars[0].advance, 16);
        assert_eq!(font.chars[1].code, 0x21);
        assert_eq!(font.chars[1].advance, 6);
        assert_eq!(font.chars[2].code, 0x22);
        assert_eq!(font.chars[2].advance, 9);
    }

    #[test]
    fn test_parse_font_tiny_with_meta() {
        let mut data = MAGIC.to_vec();
        // Meta: count=256, height=13, unknown=16
        data.extend_from_slice(&[0x00, 0x01, 0x0d, 0x00, 0x10, 0x00]);
        // Just 3 characters + terminator (not full 256 for test brevity)
        for code in 0..3u8 {
            data.extend_from_slice(&[code, 0x09, 0x00]); // width=9
        }
        data.extend_from_slice(&[0x00, 0x00, 0x00]);

        let font = BmpfFont::parse(&data).unwrap();
        assert_eq!(font.height, 13);
        assert_eq!(font.chars.len(), 3);
        assert_eq!(font.chars[0].advance, 9);
    }

    #[test]
    fn test_reject_bad_magic() {
        let data = b"not a bmpf file";
        let result = BmpfFont::parse(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_bmfont() {
        let atlas = FontAtlas {
            texture_w: 256,
            texture_h: 64,
            line_height: 16,
            base: 13,
            glyphs: vec![
                GlyphRegion {
                    code: 65, x: 0, y: 0, w: 8, h: 12,
                    x_advance: 10.0, x_offset: 0, y_offset: 1,
                },
                GlyphRegion {
                    code: 66, x: 10, y: 0, w: 7, h: 12,
                    x_advance: 9.0, x_offset: 0, y_offset: 1,
                },
            ],
        };

        let fnt = generate_bmfont(&atlas, "test", "../textures/test.png");
        assert!(fnt.contains(r#"face="test""#));
        assert!(fnt.contains("size=16"));
        assert!(fnt.contains("lineHeight=16 base=13 scaleW=256 scaleH=64"));
        assert!(fnt.contains(r#"file="../textures/test.png""#));
        assert!(fnt.contains("chars count=2"));
        assert!(fnt.contains("char id=65 x=0 y=0 w=8 h=12 xoffset=0 yoffset=1 xadvance=10"));
        assert!(fnt.contains("char id=66 x=10 y=0 w=7 h=12 xoffset=0 yoffset=1 xadvance=9"));
    }
}
