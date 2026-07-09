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

use std::fmt::Write;

use crate::atlas::FontAtlas;

pub fn generate_bmfont(font: &FontAtlas, name: &str, texture_rel_path: &str) -> String {
    let mut out = String::new();

    let _ = writeln!(
        out,
        r#"info face="{}" size={} bold=0 italic=0 charset="" unicode=0 stretchH=100 smooth=1 aa=1 padding=0,0,0,0 spacing=0,0"#,
        name, font.line_height,
    );

    let _ = writeln!(
        out,
        "common lineHeight={} base={} scaleW={} scaleH={} pages=1 packed=0",
        font.line_height, font.base, font.texture_w, font.texture_h,
    );

    let _ = writeln!(out, r#"page id=0 file="{}""#, texture_rel_path);

    let visible: Vec<&crate::atlas::GlyphRegion> = font.glyphs.iter().collect();
    let _ = writeln!(out, "chars count={}", visible.len());

    for g in &visible {
        let _ = writeln!(
            out,
            "char id={} x={} y={} w={} h={} xoffset={} yoffset={} xadvance={} page=0 chnl=0",
            g.code as u32, g.x, g.y, g.w, g.h, g.x_offset, g.y_offset, g.x_advance as u32,
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::{FontAtlas, GlyphRegion};

    #[test]
    fn test_generate_bmfont() {
        let atlas = FontAtlas {
            texture_w: 256,
            texture_h: 64,
            line_height: 16,
            base: 13,
            glyphs: vec![
                GlyphRegion {
                    code: 65,
                    x: 0,
                    y: 0,
                    w: 8,
                    h: 12,
                    x_advance: 10.0,
                    x_offset: 0,
                    y_offset: 1,
                },
                GlyphRegion {
                    code: 66,
                    x: 10,
                    y: 0,
                    w: 7,
                    h: 12,
                    x_advance: 9.0,
                    x_offset: 0,
                    y_offset: 1,
                },
            ],
        };

        let fnt = generate_bmfont(&atlas, "test", "../textures/test.png");
        assert!(fnt.contains(r#"face="test""#));
        assert!(fnt.contains("size=16"));
        assert!(fnt.contains("lineHeight=16 base=13 scaleW=256 scaleH=64"));
        assert!(fnt.contains(r#"file="../textures/test.png""#));
        assert!(fnt.contains("chars count=2"));
        assert!(fnt.contains(
            "char id=65 x=0 y=0 w=8 h=12 xoffset=0 yoffset=1 xadvance=10"
        ));
        assert!(fnt.contains(
            "char id=66 x=10 y=0 w=7 h=12 xoffset=0 yoffset=1 xadvance=9"
        ));
    }
}
