//! Command-line options and help text.

use std::path::PathBuf;

const ANALYSES: &[&str] = &[
    "authenticity",
    "bit-depth",
    "spectrum",
    "stereo",
    "phase",
    "hf-stereo",
    "clipping",
    "loudness",
    "dynamics",
    "clicks",
    "dropouts",
    "dc-offset",
    "flac-md5",
    "fingerprints",
];

#[derive(Default)]
pub(crate) struct Args {
    pub(crate) targets: Vec<String>,
    pub(crate) analyses: Vec<String>,
    pub(crate) json: Option<PathBuf>,
    pub(crate) json_layout: Option<String>,
    pub(crate) force: bool,
    pub(crate) show_results: bool,
    pub(crate) recursive: bool,
    pub(crate) verify_flac_md5: bool,
    pub(crate) ffmpeg: Option<String>,
}

impl Args {
    pub(crate) fn parse(raw: impl IntoIterator<Item = String>) -> Result<Option<Self>, String> {
        let mut args = Args {
            recursive: true,
            verify_flac_md5: true,
            ..Args::default()
        };
        let mut values = raw.into_iter();
        while let Some(arg) = values.next() {
            if arg == "--" {
                args.targets.extend(values);
                break;
            }
            let (option, inline) = arg.split_once('=').unwrap_or((&arg, ""));
            match option {
                "--help" | "-h" => return Ok(None),
                "--analysis" | "-a" => {
                    let value = argument_value(option, inline, &mut values)?;
                    for name in value.split(',') {
                        if !ANALYSES.contains(&name) {
                            return Err(format!(
                                "unknown analysis '{name}'; choose from {}",
                                ANALYSES.join(", ")
                            ));
                        }
                        if !args.analyses.iter().any(|selected| selected == name) {
                            args.analyses.push(name.to_string());
                        }
                    }
                }
                "--json" | "-j" => {
                    args.json = Some(PathBuf::from(argument_value(option, inline, &mut values)?));
                }
                "--json-layout" => {
                    let layout = argument_value(option, inline, &mut values)?;
                    if !matches!(layout.as_str(), "album" | "artist") {
                        return Err("--json-layout must be album or artist".into());
                    }
                    args.json_layout = Some(layout);
                }
                "--force" if inline.is_empty() => args.force = true,
                "--show-results" if inline.is_empty() => args.show_results = true,
                "--ffmpeg" => args.ffmpeg = Some(argument_value(option, inline, &mut values)?),
                "--recursive" if inline.is_empty() => args.recursive = true,
                "--no-recursive" if inline.is_empty() => args.recursive = false,
                "--no-flac-md5" if inline.is_empty() => args.verify_flac_md5 = false,
                _ if arg.starts_with('-') => return Err(format!("unknown option '{arg}'")),
                _ => args.targets.push(arg),
            }
        }
        if args.json.is_some() && args.json_layout.is_some() {
            return Err("choose --json PATH or --json-layout, not both".into());
        }
        if args.targets.is_empty() {
            return Err("give at least one audio file or folder".to_string());
        }
        Ok(Some(args))
    }

    pub(crate) fn selected(&self, name: &str) -> bool {
        self.analyses.is_empty() || self.analyses.iter().any(|selected| selected == name)
    }
}

fn argument_value(
    option: &str,
    inline: &str,
    values: &mut impl Iterator<Item = String>,
) -> Result<String, String> {
    let value = if inline.is_empty() {
        values
            .next()
            .ok_or_else(|| format!("{option} needs a value"))?
    } else {
        inline.to_string()
    };
    if value.is_empty() || value.starts_with('-') {
        return Err(format!("{option} needs a value"));
    }
    Ok(value)
}

pub(crate) fn help() {
    println!(
        "FlacCompagnon {}\nUsage: flaccompagnon [OPTIONS] <FILE|FOLDER>...\n\nOptions:",
        env!("CARGO_PKG_VERSION")
    );
    for (option, description) in [
        (
            "-a, --analysis NAME",
            "Select terminal fields (repeatable; implies --show-results)",
        ),
        (
            "--show-results",
            "Print analysis results (default: progress only)",
        ),
        ("-j, --json PATH", "Save one full JSON report at PATH"),
        (
            "--json-layout album|artist",
            "Save <album>.json in each album or its parent folder",
        ),
        ("--force", "Reanalyze files and replace selected reports"),
        ("--no-recursive", "Scan only the named folder level"),
        (
            "--no-flac-md5",
            "Read FLAC signatures without verifying them",
        ),
        ("--ffmpeg PATH", "Use ffmpeg for DSD content analysis"),
        ("-h, --help", "Show this help"),
        ("-v, -V, --version", "Show the version"),
    ] {
        println!("  {option:<29} {description}");
    }
    println!("\nAnalyses: {}\n\nJSON always contains the full report. Existing valid results are reused.\nAlbum grouping uses audio folders; CD/Disc subfolders share their parent.", ANALYSES.join(", "));
}

#[cfg(test)]
#[path = "../tests/unit/args.rs"]
mod tests;
