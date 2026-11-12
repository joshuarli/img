# Rimage

[![build status](https://img.shields.io/github/actions/workflow/status/SalOne22/rimage/rimage.yml?label=rimage&style=flat-square)](https://github.com/SalOne22/rimage/actions?query=branch%3Amain+)
[![docs.rs](https://img.shields.io/docsrs/rimage/latest?style=flat-square)](https://docs.rs/rimage)
[![version](https://img.shields.io/crates/v/rimage?style=flat-square)](https://crates.io/crates/rimage)
[![license](https://img.shields.io/crates/l/rimage?style=flat-square)](https://github.com/SalOne22/rimage)

A powerful Rust image optimization CLI tool inspired by [squoosh!](https://squoosh.app/).

## Features

- Modern codecs:
  - Rimage uses modern codecs optimized to produce tiny images
  - Under the hood uses `zune_image` crate that enhances performance
- Optimization operations:
  - Rimage provides several image optimization operation
  - Resize - uses `fast_image_resize` crate that has incredible performance
  - Quantization - allowing to reduce image palette
- Friendly output:
  - Rimage support progressbar
  - Rimage would show detailed error info to assist users
- CJK and Punctuation marks support:
  - Rimage supports full CJK (Chinese, Japanese and Korean) characters input and output
  - Rimage allows special punctuation characters such as `|`, ` `, `&`, `$`, etc. to be included in file names

## Installation

You can download latest release from the [releases](https://github.com/SalOne22/rimage/releases) tab.

If you're a Rust programmer, rimage can be installed with `cargo`.

```sh
cargo install rimage
```

Alternatively, one can use [cargo binstall](https://github.com/cargo-bins/cargo-binstall) to install a rimage binary directly from GitHub:

```sh
cargo binstall rimage
```

> ### Note
>
> If you're a user who just want to **use Rimage easily with a friendly GUI**, [Rimage_gui](https://github.com/Mikachu2333/rimage_gui/releases/) may be fit for you, it support both Chinese and English. Just select the version you need and download it to use.

## Usage

Run `rimage --help` for all commands and options. Run `rimage <command> --help` for a command's options.


### Basic optimization suitable for web

To optimize images with great defaults, you can simply call `rimage <command>`. For example:

```sh
rimage mozjpeg ./image.jpg
```

By default rimage will place output images right in place of precious images, resulting in overwrite if input and output has the same format. To change this behavior you can use this options:

```sh
# will place output images in `./output` directory, images may be overwritten if has the same name
rimage mozjpeg -d ./output ./image.jpg

# will keep the original image as `<name>@backup.<ext>` next to the input
rimage mozjpeg --backup ./image.jpg

# will place output images in ./output directory preserving folder structure
rimage mozjpeg -d ./output -r ./inner/image.jpg ./image.jpg
```

### Color fidelity and chroma subsampling

`rimage mozjpeg` lets MozJPEG choose the chroma subsampling automatically. At
typical quality levels it picks **2x2 (4:2:0)**, which discards three quarters
of the chroma resolution to make the file smaller. A side effect is that highly
saturated colors (for example a bright red) can come out visibly darker or
duller than the original.

If color fidelity matters more than file size, force 4:4:4 with:

```sh
rimage mozjpeg --subsample 1 ./image.jpg
```

`1` means 4:4:4 (no chroma subsampling), `2` means 4:2:0. The default
quantization table is `NRobidoux`; if you are comparing output with tools that
use the standard Annex K tables (such as ImageMagick), pass
`--qtable AnnexK` to make the comparison closer.

### File lists

To process many images without a long command line, create a UTF-8 text file named `file.list`
with one input file per line and pass it as an input:

```sh
rimage mozjpeg file.list
```

Blank lines are skipped, surrounding whitespace is ignored, and relative paths are resolved
against the current working directory. Glob patterns are supported on each line. When a
`file.list` is provided, all other input file arguments are ignored.

### Preprocessing

Rimage supports a preprocessing pipeline: resize, color quantization, and alpha premultiply
run before encoding. Operations execute in CLI argument order.

#### Resize

`--resize` accepts one of the following value forms. Unless a fixed `WxH` is
given, the aspect ratio is preserved:

| Form  | Meaning                                               | Example   | Result on 800x400 |
| ----- | ----------------------------------------------------- | --------- | ----------------- |
| `WxH` | Fixed width and height, aspect ratio is not preserved | `100x300` | `100x300`         |
| `Ww`  | Anchor on the width, height follows the aspect ratio  | `100w`    | `100x50`          |
| `hH`  | Anchor on the height, width follows the aspect ratio  | `200h`    | `400x200`         |
| `Ll`  | Longest side becomes `L`, the other side follows      | `1000l`   | `1000x500`        |
| `Ss`  | Shortest side becomes `S`, the other side follows     | `500s`    | `1000x500`        |
| `@M`  | Scale by the multiplier `M`                           | `@1.5`    | `1200x600`        |
| `P%`  | Scale to `P` percent of the source                    | `50%`     | `400x200`         |

```sh
# Fixed dimensions: the image is resized to exactly 100x200
rimage mozjpeg --resize 100x200 ./image.jpg     # 800x400 -> 100x200

# Anchor on one side, the other side follows the aspect ratio
rimage mozjpeg --resize 100w ./image.jpg        # 800x400 -> 100x50
rimage mozjpeg --resize 200h ./image.jpg        # 800x400 -> 400x200

# Longest / shortest side: the anchor is chosen per image, so portrait and
# landscape images in the same batch end up with a consistent size
rimage mozjpeg --resize 1000l ./landscape.jpg   # 800x400 -> 1000x500
rimage mozjpeg --resize 1000l ./portrait.jpg    # 400x800 -> 500x1000
rimage mozjpeg --resize 500s ./landscape.jpg    # 800x400 -> 1000x500
rimage mozjpeg --resize 500s ./portrait.jpg     # 400x800 -> 500x1000

# Multiplier and percentage
rimage mozjpeg --resize @2 ./image.jpg          # 800x400 -> 1600x800
rimage mozjpeg --resize @0.5 ./image.jpg        # 800x400 -> 400x200
rimage mozjpeg --resize 50% ./image.jpg         # 800x400 -> 400x200
rimage mozjpeg --resize 150% ./image.jpg        # 800x400 -> 1200x600
```

Passing `--resize` several times chains the values. Each value maps the size
the previous resize produced, so the order of the values matters:

```sh
# 100x400 first, then the shortest side of that intermediate result
rimage mozjpeg --resize 100x400 --resize 200s ./image.jpg   # 800x400 -> 200x800

# Longest side first, then half of the intermediate result
rimage mozjpeg --resize 1000l --resize 50% ./image.jpg      # 800x400 -> 500x250
```

The direction flags restrict each resize step to shrinking or growing only.
`--reduce-only` is an alias for `--no-upscale` (never grow) and `--enlarge-only`
is an alias for `--no-downscale` (never shrink):

```sh
# Shrink images whose longest side is above 1000px, leave the rest untouched
rimage mozjpeg --resize 1000l --reduce-only ./small.jpg     # 800x400 -> 800x400
rimage mozjpeg --resize 1000l --reduce-only ./big.jpg       # 2000x1000 -> 1000x500

# Grow images whose longest side is below 1000px, leave the rest untouched
rimage mozjpeg --resize 1000l --enlarge-only ./small.jpg    # 800x400 -> 1000x500
rimage mozjpeg --resize 1000l --enlarge-only ./big.jpg      # 2000x1000 -> 2000x1000

# A step that is not allowed by the flags is skipped, and the chain continues
# from the size the image actually has
rimage mozjpeg --resize 2000l --resize 50% --reduce-only ./image.jpg  # 800x400 -> 400x200
```

> **Note**: Passing both `--reduce-only` and `--enlarge-only` leaves no direction to resize
> in, so every image keeps its original size. The order of the two flags makes no difference.

`--filter` selects the resampling filter applied to every `--resize` step
(default `lanczos3`). Available filters: `nearest`, `box`, `bilinear`,
`hamming`, `catmull-rom`, `mitchell`, `lanczos3`.

```sh
rimage mozjpeg --resize 1000l --filter nearest ./image.jpg
```

#### Quantization (color palette reduction)

`--quantization` reduces the number of distinct colors in the image. It is **not** a
substitute for `-q`/`--quality` — quantization limits the color palette, while
`-q` controls encoder compression.

```sh
# Quantize to 80% palette quality, then encode at default JPEG quality 75
rimage mozjpeg --quantization 80 ./image.jpg

# For best compression, combine with a lower quality value
rimage mozjpeg -q 50 --quantization 80 ./image.jpg
```

> **Note**: Using `--quantization` without lowering `-q` may produce files nearly as large
> as without quantization, because sharp palette boundaries (banding) are faithfully
> reproduced by the encoder at high quality settings.

#### Pipeline ordering

Preprocessing operations run in the order they appear on the command line:

```sh
# Quantize first, then resize to 64x64 (nearest filter)
rimage mozjpeg --quantization 80 --resize 64x64 --filter nearest ./image.jpg

# Resize first, then quantize
rimage mozjpeg --resize 64x64 --filter nearest --quantization 80 ./image.jpg
```

Note that `--filter` applies to all `--resize` invocations, and `--dithering`
applies to all `--quantization` invocations.

### Advanced options

Each encoder accepts additional options. For example, `rimage mozjpeg --help` lists JPEG quality, chroma subsampling, and quantization-table settings.

For more info use `rimage help <command>`, e.g. `rimage help mozjpeg`

For library usage check [Docs.rs](https://docs.rs/rimage/latest/rimage/)

### List of supported Codecs

| Image Codecs | Decoder       | Encoder                 | NOTE                                                 |
| ------------ | ------------- | ----------------------- | ---------------------------------------------------- |
| avif         | dav1d         | ravif                   | Common features only, Static only                    |
| bmp          | zune-bmp      | ❌                      | Input only                                           |
| farbfeld     | zune-farbfeld | zune-farbfeld           |                                                      |
| hdr          | zune-hdr      | zune-hdr                |                                                      |
| jpeg         | zune-jpeg     | mozjpeg or jpeg-encoder | Multifunctional when use mozjpeg encoder             |
| jpeg-xl      | jxl-oxide     | zune-jpegxl             | Lossless only, Static only                           |
| png          | zune-png      | oxipng or zune-png      | Static only, Multifunctional when use oxipng encoder |
| ppm          | zune-ppm      | zune-ppm                |                                                      |
| psd          | zune-psd      | ❌                      | Input only                                           |
| qoi          | zune-qoi      | zune-qoi                |                                                      |
| tiff         | tiff          | ❌                      | Input only                                           |
| webp         | webp          | webp                    | Static only                                          |

### List of supported preprocessing options

- Resize
- Quantization
- Alpha premultiply

## List of supported mode for output info presenting

- No-progress (Shown on Default)
- Quiet (Show all msgs on Default)

## Known bugs & Warnings

- PSD is partially supported; images with ICC profiles may have incorrect color.

## Build

Install Rust, CMake, NASM, pkg-config, and dav1d (version 1.3.0 or later), then run:

```sh
cargo build --all-features
make test
```

For Linux testing with the Alpine musl toolchain:

```sh
docker build -t rimage-linux .
docker run --rm -v "$PWD:/rimage" -v rimage-linux-target:/rimage/target rimage-linux
```

The image installs the toolchain from `rust-toolchain.toml`. Rebuild the image after changing it.

## Contributing

Read the [contribution guide](CONTRIBUTING.md) for build instructions and guidelines.

## License

Rimage is dual-licensed under [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0) and [MIT License](https://opensource.org/licenses/MIT). You can choose either license for your use.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Changelog

View the [Changelog](CHANGELOG.md) for version-specific changes.
