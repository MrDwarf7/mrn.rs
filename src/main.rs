mod cli;
mod consts;
mod macros;
mod prelude;

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use colored::*;
use image::{ImageError, ImageResult, image_dimensions};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::cli::Cli;
use crate::consts::{RE_CLEAN, TRIM_CHARS};
pub use crate::prelude::*;

#[derive(Debug, Clone, Copy)]
struct Width(u32);

#[derive(Debug, Clone, Copy)]
struct Height(u32);

#[derive(Debug, Clone, Copy)]
struct Dimensions {
    width:  Width,
    height: Height,
}

impl TryFrom<&Path> for Dimensions {
    type Error = ImageError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let (w, h) = image_dimensions(path).map_err(|e| {
            eprintln!(
                "Failed to get dimensions for {}: {}",
                path.display(),
                format!("{}", e).red().bold()
            );
            e
        })?;

        Ok(Dimensions {
            width:  Width(w),
            height: Height(h),
        })
    }
}

impl TryFrom<ImageResult<(u32, u32)>> for Dimensions {
    type Error = ImageError;

    fn try_from(value: ImageResult<(u32, u32)>) -> Result<Self, Self::Error> {
        value.map(|(w, h)| {
            Dimensions {
                width:  Width(w),
                height: Height(h),
            }
        })
    }
}

impl std::fmt::Display for Dimensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width.0, self.height.0)
    }
}

#[derive(Debug)]
struct ImageCandidate<P: AsRef<Path> = PathBuf, S: AsRef<str> = String> {
    path:         P,
    cleaned_name: S,
    dims:         Dimensions,
    extension:    S,
}

impl ImageCandidate<PathBuf, String> {
    fn new<P, S>(path: P, cleaned_name: S, dims: Dimensions, extension: S) -> Self
    where
        P: AsRef<Path>,
        S: Into<String>,
    {
        Self {
            path: path.as_ref().to_path_buf(),
            cleaned_name: cleaned_name.into(),
            dims,
            extension: extension.into(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Behaviour encoded in types – zero bools, zero runtime decisions for copy/move
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Debug)]
enum TargetMode<P: AsRef<Path>> {
    InPlace,
    CopyTo(P),
    MoveTo(P),
}

impl From<&crate::cli::Cli> for TargetMode<PathBuf> {
    fn from(value: &Cli) -> Self {
        match (&value.output, value.r#move) {
            (Some(p), true) => TargetMode::MoveTo(p.to_owned()),
            (Some(p), false) => TargetMode::CopyTo(p.to_owned()),
            (None, _) => TargetMode::InPlace,
        }
    }
}

#[derive(Debug)]
struct Relocation<P: AsRef<Path> = PathBuf> {
    from: P,
    to:   P,
}

#[derive(Debug)]
enum Action {
    Rename(Relocation),
    Copy(Relocation),
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Rename(r) => write!(f, "-> {} -> {}", r.from.display(), r.to.display()),
            Action::Copy(r) => write!(f, "*C {} -> {}", r.from.display(), r.to.display()),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────
fn main() -> Result<()> {
    let args = Cli::new();
    let mode = TargetMode::from(&args);

    println!(
        "{} Scanning {} ({} extensions, follow symlinks: {})",
        "S".cyan(),
        args.input.display(),
        args.exts.len(),
        args.follow_links
    );

    let candidates = collect_candidates(&args.input, &args.exts, args.follow_links);

    let groups: HashMap<String, Vec<ImageCandidate>> =
        candidates.into_iter().fold(HashMap::new(), |mut acc, c| {
            acc.entry(c.cleaned_name.clone()).or_default().push(c);
            acc
        });

    let actions: Vec<Action> = groups
        .into_values()
        .flat_map(|group| build_actions(group, &mode))
        .collect();

    args.dry_run(&actions);

    execute_actions(&actions)?;

    println!("Done — {} files processed", format!("{}", actions.len()).green());
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Clean, readable collection (no 8-line chain in main)
// ─────────────────────────────────────────────────────────────────────────────
fn collect_candidates(input: &Path, exts: &[String], follow_links: bool) -> Vec<ImageCandidate> {
    WalkDir::new(input)
        .follow_links(follow_links)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .collect::<Vec<_>>()
        .into_par_iter()
        .filter_map(|entry| {
            let path = entry.into_path();
            is_supported_extension(&path, exts)
                .unwrap_or(false)
                .then(|| process_file(path))
                .flatten()
            // if !is_supported_extension(&path, exts) {
            //     return None;
            // }
            // process_file(path)
        })
        .collect()
}

fn is_supported_extension<P: AsRef<Path>>(path: P, exts: &[String]) -> Option<bool> {
    let ext = path.as_ref().extension().and_then(|s| s.to_str())?;
    Some(exts.iter().any(|e| e.eq_ignore_ascii_case(ext)))
}

fn process_file<P: AsRef<Path>>(path: P) -> Option<ImageCandidate> {
    // let dims = imagesize::size(&path).and_then(Dimensions::<>try_from).ok()?;
    let path = path.as_ref();

    let stem = path.file_stem()?.to_str()?.to_string();
    let cleaned_name = clean_stem(&stem);

    let ext = path.extension()?.to_str()?.to_lowercase();

    // Both are valid and impl'd.
    //
    // let dims = Dimensions::try_from(image_dimensions(path)).ok()?;
    let dims = Dimensions::try_from(path).ok()?;
    Some(ImageCandidate::new(path, cleaned_name, dims, ext.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Pure, allocation-free cleaning
// ─────────────────────────────────────────────────────────────────────────────
fn clean_stem(stem: &'_ str) -> Cow<'_, str> {
    let trimmed = RE_CLEAN
        .replace_all(stem, "")
        .trim_matches(TRIM_CHARS)
        .to_string();

    if trimmed.is_empty() {
        // stem.to_string()
        Cow::Borrowed(stem)
    } else {
        // trimmed.to_string()
        Cow::Owned(trimmed)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Behaviour fully encoded — no if/else inside the loop
// ─────────────────────────────────────────────────────────────────────────────
fn build_actions(mut group: Vec<ImageCandidate>, mode: &TargetMode<PathBuf>) -> Vec<Action> {
    let base = match &mode {
        TargetMode::InPlace => None,
        TargetMode::CopyTo(p) | TargetMode::MoveTo(p) => Some(p),
    };

    // We only need to sort when there are duplicates (rare)
    if group.len() > 1 {
        group.sort_by_key(|c| c.dims.width.0 * c.dims.height.0);
    }

    group
        .into_iter()
        .filter_map(|c| {
            let new_stem = format!("{}-[{}]", c.cleaned_name, c.dims);

            // Skip already-perfect files when doing in-place (single item only)
            if let TargetMode::InPlace = mode
                && c.path.file_stem().and_then(|s| s.to_str()) == Some(&new_stem)
            {
                return None;
            }

            let target_base = base
                .as_ref()
                .map(|b| b.as_path())
                .unwrap_or_else(|| c.path.parent().unwrap_or_else(|| Path::new("")));

            let to = unique_target_path(target_base, &new_stem, &c.extension)
                .as_ref()
                .to_path_buf();

            let relocation = Relocation { from: c.path, to };

            let action = match mode {
                TargetMode::InPlace | TargetMode::MoveTo(_) => Action::Rename(relocation),
                TargetMode::CopyTo(_) => Action::Copy(relocation),
            };
            Some(action)
        })
        .collect()
}

fn unique_target_path<P, S>(base: P, stem: S, ext: S) -> impl AsRef<Path>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    std::iter::once(0u32)
        .map(|n| {
            let ext = ext.as_ref();
            let stem = stem.as_ref();
            #[rustfmt::skip]
            let name = if n == 0 {
                format!("{}.{}", stem, ext)
            } else { format!("{}[{}].{}", stem, n, ext) };
            base.as_ref().join(name)
        })
        .find(|p| !p.exists())
        .expect("collision loop impossible")
}

fn execute_actions(actions: &[Action]) -> Result<()> {
    let mut writer = io::BufWriter::new(io::stdout().lock());

    for action in actions {
        let msg = match action {
            Action::Rename(r) => {
                std::fs::create_dir_all(
                    r.to.parent()
                        .expect("Target path must have a parent directory"),
                )
                .expect("Failed to create target directory");

                std::fs::rename(&r.from, &r.to).map(|_| "Renamed")
            }
            Action::Copy(r) => {
                let _ = std::fs::create_dir_all(r.to.parent().unwrap());
                std::fs::copy(&r.from, &r.to).map(|_| "Copied")
            }
        };

        if let Ok(m) = msg {
            writeln!(writer, "{}", m)?;
        }
    }
    writer.flush()?;
    Ok(())
}
