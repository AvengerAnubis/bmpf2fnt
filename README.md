# bmpf2fnt

Convert Stranded II / Blitz3D `.bmpf` bitmap font files to BMFont `.fnt` format.

## Usage

```text
bmpf2fnt <bmpf> <bmp>... [output]
```

- `<bmpf>` – the `.bmpf` font definition file
- `<bmp>...` – one or more `.bmp` font atlas textures
- `[output]` – optional output directory (default: current directory)

Each `.bmp` produces a corresponding `.fnt` file with the same base name.

### Example

```sh
bmpf2fnt font.bmpf font.bmp
# produces ./font.fnt

bmpf2fnt font.bmpf font1.bmp font2.bmp output/
# produces output/font1.fnt and output/font2.fnt
```

## Library

The crate also provides a library API:

```rust
use bmpf2fnt::{BmpfFont, build_font_atlas, generate_bmfont};

let font_data = std::fs::read("font.bmpf")?;
let font = BmpfFont::parse(&font_data)?;

let img = image::open("font.bmp")?.into_rgba8();
let (w, h) = img.dimensions();
let atlas = build_font_atlas(img.as_raw(), w, h, &font)?;

let fnt = generate_bmfont(&atlas, "font.bmp", "font.bmp");
std::fs::write("font.fnt", &fnt)?;
```

### Modules

| Module   | Description                           |
|----------|---------------------------------------|
| `bmpf`   | `.bmpf` file parsing                  |
| `atlas`  | Glyph region detection and atlas building |
| `fnt`    | BMFont `.fnt` text generation         |
| `cli`    | Command-line interface                |
| `error`  | Error types                           |

## `.bmpf` file format

```text
[46-byte ASCII header]  "Unreal Software Bitmap Font Wizard bmpf File\r\n"
[optional 6-byte meta]   only present in full (256-char) fonts:
  u16 LE: char_count     (256)
  u16 LE: font_height    (pixels)
  u16 LE: unknown
[3-byte records …]
  u8:    character code
  u16 LE: advance width
[terminator]             00 00 00
```

## License

GPL-3.0-or-later. See [`LICENSE`](LICENSE).
