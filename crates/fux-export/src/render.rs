//! PDF, SVG and PNG through the `lilypond` program.

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

/// Graphic formats LilyPond can produce.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Graphic {
    Pdf,
    Svg,
    Png,
}

impl Graphic {
    fn flag(self) -> &'static str {
        match self {
            Graphic::Pdf => "--pdf",
            Graphic::Svg => "--svg",
            Graphic::Png => "--png",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Graphic::Pdf => "pdf",
            Graphic::Svg => "svg",
            Graphic::Png => "png",
        }
    }
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error(
        "LilyPond was not found ({0}). Install it from https://lilypond.org or set FUX_LILYPOND to the program."
    )]
    Missing(String),
    #[error("LilyPond failed:\n{0}")]
    Failed(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// The LilyPond program: `$FUX_LILYPOND` or `lilypond` from the path.
pub fn lilypond_command() -> String {
    std::env::var("FUX_LILYPOND").unwrap_or_else(|_| "lilypond".to_string())
}

/// The version line of the installed LilyPond, if there is one.
pub fn lilypond_version() -> Option<String> {
    let output = Command::new(lilypond_command())
        .arg("--version")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().next().map(str::to_string)
}

/// Compiles `ly_file` into `format`. The output is named after `output_base` with the format's
/// extension; for several pages LilyPond numbers the SVG and PNG files. Returns the created files.
pub fn render(
    ly_file: &Path,
    output_base: &Path,
    format: Graphic,
) -> Result<Vec<PathBuf>, RenderError> {
    let program = lilypond_command();
    let mut command = Command::new(&program);
    command.arg("-dno-point-and-click").arg(format.flag());
    if format == Graphic::Png {
        command.arg("-dresolution=150");
    }
    // Options must come before the input file.
    command.arg("-o").arg(output_base).arg(ly_file);
    let output = command.output().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => RenderError::Missing(program.clone()),
        _ => RenderError::Io(e),
    })?;
    if !output.status.success() {
        return Err(RenderError::Failed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    created_files(output_base, format.extension())
}

/// Files LilyPond wrote for a base name: `base.ext`, `base-1.ext`, `base-page1.ext`, …
fn created_files(base: &Path, extension: &str) -> Result<Vec<PathBuf>, RenderError> {
    let dir = match base.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let stem = base
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned());
            name.is_some_and(|n| {
                n.ends_with(&format!(".{extension}"))
                    && (n == format!("{stem}.{extension}") || n.starts_with(&format!("{stem}-")))
            })
        })
        .collect();
    files.sort();
    Ok(files)
}
