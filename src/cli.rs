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

use std::path::{Path, PathBuf};

use crate::atlas::build_font_atlas;
use crate::bmpf::BmpfFont;
use crate::error::BmpfError;
use crate::fnt::generate_bmfont;

const USAGE: &str = "\
Usage: bmpf2fnt <bmpf> <bmp>... [output]

Convert Stranded II .bmpf bitmap font definition and accompanying
.bmp texture(s) into BMFont .fnt format files.

Arguments:
  <bmpf>       Path to the .bmpf font definition file
  <bmp>...     One or more .bmp texture files
  [output]     Optional output directory (default: current directory)

Options:
  --help, -?   Show this help message and exit
";

struct Args {
    help: bool,
    bmpf: PathBuf,
    bmps: Vec<PathBuf>,
    output_dir: PathBuf,
}

fn parse_args() -> Result<Args, String> {
    let raw: Vec<String> = std::env::args().collect();

    if raw.len() >= 2 && (raw[1] == "--help" || raw[1] == "-?") {
        return Ok(Args {
            help: true,
            bmpf: PathBuf::new(),
            bmps: vec![],
            output_dir: PathBuf::new(),
        });
    }

    if raw.len() < 3 {
        return Err(format!("too few arguments\n\n{USAGE}"));
    }

    let bmpf = PathBuf::from(&raw[1]);
    let mut bmp_args: Vec<PathBuf> = raw[2..].iter().map(PathBuf::from).collect();

    // If the last argument is NOT an existing regular file, treat it as
    // the output directory (possibly creating it later).
    let output_dir = if bmp_args.len() >= 2 && !bmp_args.last().unwrap().is_file() {
        bmp_args.pop().unwrap()
    } else {
        PathBuf::from(".")
    };

    if !bmpf.is_file() {
        return Err(format!("file not found: {}", bmpf.display()));
    }
    for b in &bmp_args {
        if !b.is_file() {
            return Err(format!("file not found: {}", b.display()));
        }
    }

    Ok(Args {
        help: false,
        bmpf,
        bmps: bmp_args,
        output_dir,
    })
}

pub fn run() -> Result<(), BmpfError> {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    if args.help {
        print!("{USAGE}");
        return Ok(());
    }

    let bmpf_data = std::fs::read(&args.bmpf)
        .map_err(|e| BmpfError::Io(format!("cannot read {}: {e}", args.bmpf.display())))?;
    let font_def = BmpfFont::parse(&bmpf_data)?;

    for bmp_path in &args.bmps {
        convert_one(bmp_path, &font_def, &args.output_dir)?;
    }

    Ok(())
}

fn convert_one(bmp_path: &Path, font_def: &BmpfFont, out_dir: &Path) -> Result<(), BmpfError> {
    let img = image::open(bmp_path)
        .map_err(|e| BmpfError::Io(format!("cannot open {}: {e}", bmp_path.display())))?;
    let img = img.into_rgba8();
    let (w, h) = img.dimensions();
    let atlas = build_font_atlas(img.as_raw(), w, h, font_def)?;

    let stem = bmp_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("font");

    let name = bmp_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(stem);

    let fnt_content = generate_bmfont(&atlas, name, name);

    std::fs::create_dir_all(out_dir)
        .map_err(|e| BmpfError::Io(format!("cannot create output dir: {e}")))?;

    let mut out_path = out_dir.to_path_buf();
    out_path.push(format!("{stem}.fnt"));

    std::fs::write(&out_path, &fnt_content)
        .map_err(|e| BmpfError::Io(format!("cannot write {}: {e}", out_path.display())))?;

    eprintln!("wrote {}", out_path.display());
    Ok(())
}
