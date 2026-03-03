# mrn

[![Build](https://github.com/mrdwarf7/mrn.rs/actions/workflows/build.yml/badge.svg)](https://github.com/mrdwarf7/mrn.rs/actions/workflows/build.yml)
[![Test](https://github.com/mrdwarf7/mrn.rs/actions/workflows/test.yml/badge.svg)](https://github.com/mrdwarf7/mrn.rs/actions/workflows/test.yml)
[![Formatting](https://github.com/mrdwarf7/mrn.rs/actions/workflows/format.yml/badge.svg)](https://github.com/mrdwarf7/mrn.rs/actions/workflows/format.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**[M]ass [R]e-[n]amer** — a fast CLI tool that renames image files to include their pixel dimensions in the filename.

You know the drill: you've got a folder of wallpapers called things like `cool_sunset (1).jpg`, `wallhaven-abc123-1920x1080.png`, and `IMG_20250101_123456789.webp`.
You want them named sanely.

The classic `find ... -exec ... xargs ...` pipeline gets ugly fast, especially when you also need to read image dimensions, strip existing junk from filenames, and handle duplicates.
`mrn` does all of that in one shot.

```
Before:  cool_sunset (1)-[1920x1080]-12345678.jpg
After:   cool_sunset-[1920x1080].jpg
```

It scans a directory, reads actual image dimensions, strips pre-existing resolution strings and numeric noise from the filename, and renames (or copies/moves) the result into a clean `name-[WxH].ext` format.

## Usage

```sh
# Rename all images in-place in the current directory
mrn .

# Rename images in a specific directory
mrn ~/Pictures/Wallpapers

# Dry run — see what would happen without touching anything
mrn ~/Pictures/Wallpapers -d

# Copy renamed files to an output directory (originals untouched)
mrn ~/Pictures/Wallpapers -o ~/Pictures/Sorted

# Move renamed files to an output directory instead of copying
mrn ~/Pictures/Wallpapers -o ~/Pictures/Sorted -m

# Only target specific extensions
mrn ~/Pictures -E png,jpg,webp

# Follow symbolic links during the scan
mrn ~/Pictures -L
```

### CLI Reference

| Flag / Argument           | Description                                                  |
| ------------------------- | ------------------------------------------------------------ |
| `<INPUT>`                 | Directory to scan (defaults to `.`)                          |
| `-o, --output <DIR>`      | Output directory for renamed files (omit to rename in-place) |
| `-m, --move`              | Move files to the output directory instead of copying        |
| `-d, --dry_run`           | Preview actions without making any filesystem changes        |
| `-E, --extensions <LIST>` | Comma-separated list of extensions to target                 |
| `-L, --follow_links`      | Follow symbolic links while scanning                         |
| `-v, --version`           | Print version information                                    |
| `-h, --help`              | Print help                                                   |

### Supported Formats

`jpg` · `jpeg` · `png` · `webp` · `gif` · `bmp` · `tiff` · `tif` · `heic` · `avif`

These are the defaults — override with `-E` to target whatever you need.

## Pairing with Other Tools

`mrn` is deliberately single-purpose. It fits well into larger workflows:

```sh
# Sort wallpapers by resolution after renaming
mrn ~/Walls -o ~/Walls/sorted
ls ~/Walls/sorted | grep -oP '\d+x\d+' | sort -t'x' -k1 -n | uniq -c | sort -rn

# Feed into feh / swaybg / nitrogen for wallpaper rotation
mrn ~/Walls && feh --randomize --bg-fill ~/Walls/*

# Combine with fd for more targeted pre-filtering
fd -e png -e jpg . ~/Walls -x cp {} /tmp/staging/
mrn /tmp/staging -o ~/Walls/clean

# Use with rsync for remote wallpaper management
mrn ~/Walls -o ~/Walls/staged
rsync -avh ~/Walls/staged/ remote:~/Wallpapers/
```

## Internals

A few things under the hood that might be interesting if you're into Rust patterns:

### Behaviour Encoded in Types

The copy-vs-move-vs-in-place decision isn't a runtime boolean check sprinkled throughout the code.
It's a `TargetMode` enum resolved once from the CLI args, and the action-building logic is fully determined by the variant — no `if copy_mode { ... } else { ... }` in the hot path:

```rust
enum TargetMode<P: AsRef<Path>> {
    InPlace,
    CopyTo(P),
    MoveTo(P),
}
```

### Parallel Collection, Sequential Execution

File _discovery_ and dimension-reading are parallelised with [Rayon](https://github.com/rayon-rs/rayon) — the `walkdir` results are collected and then fanned out across threads via `into_par_iter()`.
The actual rename/copy/move operations run sequentially and buffered through `BufWriter` to avoid filesystem contention and keep output coherent.

### Regex-Based Filename Cleaning

Existing resolution strings (`1920x1080`, `[3840×2160]`, stray 6–10 digit numeric runs) are stripped from filenames via a compiled `LazyLock<Regex>` before the new `name-[WxH]` stem is constructed.
This means re-running `mrn` on already-renamed files won't produce `name-[1920x1080]-[1920x1080].jpg` — it converges to the correct name and skips files that are already perfect.

### Zero-Copy Where Possible

The `clean_stem` function returns `Cow<'_, str>` — if the regex doesn't match (i.e. the filename is already clean), no allocation happens. The original `&str` is borrowed through.

## Building

Requires **Rust nightly** (the project uses Edition 2024 and the Cranelift codegen backend for faster debug builds).

```sh
# Dev build (uses cranelift for fast iteration)
cargo build

# Release build
cargo build --release

# Or via cargo-make (see Makefile.toml for all tasks)
cargo make b       # dev build
cargo make br      # release build
cargo make t       # tests
cargo make f       # format
cargo make a       # everything
```

The binary is output as `mrn`.

## Contributing

Contributions are welcome. A few notes:

- You _can_ get by with plain `cargo` commands, but the project uses [cargo-make](https://github.com/sagiegurari/cargo-make) (`makers` / `ma`) with a `Makefile.toml` in the root that defines shorthand aliases for the full development workflow. I'd expect any PR to have at least been run through everything the Makefile covers before submitting. The usual dev loop is just:

  ```sh
  ma a    # format + lint + build (debug & release) + all tests — keeps everything in sync
  ma r -- --help   # run (debug)
  ma rr -- --help  # run (release)
  ```

  Other handy shortcuts: `ma t` (test), `ma f` (format), `ma b` (build), `ma br` (build release), `ma cp` (clippy). Check the `Makefile.toml` for the full list.

- Run `cargo +nightly fmt --all` and `cargo clippy --all-targets --all-features -- -W clippy::pedantic` before pushing — CI enforces both (or just `ma a` and you're covered).
- The project forbids `unsafe` code at the lint level.
- Tests live alongside the code they test (see `consts.rs` for examples).
- There's a `.github/PULL_REQUEST_TEMPLATE.md` and issue templates ready to go.

The project is _somewhat_ concurrent today — file discovery and dimension reading are parallelised via Rayon — but full end-to-end concurrency (parallel renames/copies/moves) is on the roadmap.
Probably. Eventually. (How many devs have said "I'll get round to it later" and it never happens? Yeah.)

## License

[MIT](LICENSE) © [MrDwarf7](https://github.com/MrDwarf7)

## Acknowledgements

Big thanks to the [Clap](https://github.com/clap-rs/clap) maintainers and contributors — the derive API and the styling system make building CLI tools in Rust an absolute pleasure.
Also to the [Rayon](https://github.com/rayon-rs/rayon), [image](https://github.com/image-rs/image),
and [walkdir](https://github.com/BurntSushi/walkdir) crates for doing the heavy lifting.
