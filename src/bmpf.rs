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

use crate::error::BmpfError;

const MAGIC: &[u8] = b"Unreal Software Bitmap Font Wizard bmpf File\r\n";
const MAGIC_LEN: usize = 46;

#[derive(Debug, Clone)]
pub struct BmpfChar {
    pub code: u8,
    pub advance: u16,
}

#[derive(Debug, Clone)]
pub struct BmpfFont {
    pub chars: Vec<BmpfChar>,
    pub height: u16,
}

impl BmpfFont {
    pub fn parse(data: &[u8]) -> Result<Self, BmpfError> {
        if data.len() < MAGIC_LEN {
            return Err(BmpfError::BadHeader);
        }
        if &data[..MAGIC_LEN] != MAGIC {
            return Err(BmpfError::BadMagic);
        }

        let rest = &data[MAGIC_LEN..];

        let (meta_size, height) = if rest.len() >= 6 && rest[0] == 0x00 && rest[1] == 0x01 {
            let h = u16::from_le_bytes([rest[2], rest[3]]);
            (6usize, h)
        } else {
            (0usize, 0u16)
        };

        let records_data = &rest[meta_size..];
        let mut chars = Vec::new();
        let mut i = 0;

        while i + 3 <= records_data.len() {
            if records_data[i] == 0 && records_data[i + 1] == 0 && records_data[i + 2] == 0 {
                break;
            }
            let code = records_data[i];
            let advance = u16::from_le_bytes([records_data[i + 1], records_data[i + 2]]);
            chars.push(BmpfChar { code, advance });
            i += 3;
        }

        Ok(BmpfFont { chars, height })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_font_norm() {
        let mut data = MAGIC.to_vec();
        data.extend_from_slice(&[0x65, 0x10, 0x00]);
        data.extend_from_slice(&[0x21, 0x06, 0x00]);
        data.extend_from_slice(&[0x22, 0x09, 0x00]);
        data.extend_from_slice(&[0x00, 0x00, 0x00]);

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
    fn test_parse_with_meta() {
        let mut data = MAGIC.to_vec();
        data.extend_from_slice(&[0x00, 0x01, 0x0d, 0x00, 0x10, 0x00]);
        for code in 0..3u8 {
            data.extend_from_slice(&[code, 0x09, 0x00]);
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
    fn test_parse_real_font_norm() {
        let raw = include_bytes!("../tests/in/font_norm.bmpf");
        let font = BmpfFont::parse(raw).unwrap();
        assert_eq!(font.height, 0);
        assert!(!font.chars.is_empty());
        assert_eq!(font.chars[0].code, b'e');
        // The real file encodes 'e' with advance = 0x1000 (4096 LE).
        assert_eq!(font.chars[0].advance, 4096);
    }

    #[test]
    fn test_parse_real_font_tiny() {
        let raw = include_bytes!("../tests/in/font_tiny.bmpf");
        let font = BmpfFont::parse(raw).unwrap();
        assert_eq!(font.height, 13);
        assert!(font.chars.len() > 200);
    }
}
