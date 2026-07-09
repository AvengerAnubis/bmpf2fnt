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

use std::fmt;

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

impl Clone for BmpfError {
    fn clone(&self) -> Self {
        match self {
            Self::BadHeader => Self::BadHeader,
            Self::BadMagic => Self::BadMagic,
            Self::InvalidRecord { index, message } => Self::InvalidRecord {
                index: *index,
                message: message.clone(),
            },
            Self::NoGlyphsFound => Self::NoGlyphsFound,
            Self::Mismatch {
                bmpf_chars,
                glyphs_found,
            } => Self::Mismatch {
                bmpf_chars: *bmpf_chars,
                glyphs_found: *glyphs_found,
            },
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
            Self::Mismatch {
                bmpf_chars,
                glyphs_found,
            } => {
                write!(
                    f,
                    "bmpf has {bmpf_chars} chars but found {glyphs_found} glyphs"
                )
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
