//! Standalone entry point for the same analysis and report schema as the app.

use std::path::{Path, PathBuf};

use flaccompagnon_core::{self as core, FileAnalysis, ScanOptions};

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
struct Args {
    targets: Vec<String>,
    analyses: Vec<String>,
    json: Option<PathBuf>,
    recursive: bool,
    verify_flac_md5: bool,
    ffmpeg: Option<String>,
}

impl Args {
    fn parse(raw: impl IntoIterator<Item = String>) -> Result<Option<Self>, String> {
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
                "--ffmpeg" => args.ffmpeg = Some(argument_value(option, inline, &mut values)?),
                "--recursive" if inline.is_empty() => args.recursive = true,
                "--no-recursive" if inline.is_empty() => args.recursive = false,
                "--no-flac-md5" if inline.is_empty() => args.verify_flac_md5 = false,
                _ if arg.starts_with('-') => return Err(format!("unknown option '{arg}'")),
                _ => args.targets.push(arg),
            }
        }
        if args.targets.is_empty() {
            return Err("give at least one audio file or folder".to_string());
        }
        Ok(Some(args))
    }

    fn selected(&self, name: &str) -> bool {
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

fn help() {
    println!(
        "FlacCompagnon {}\n\
         Usage: flaccompagnon [OPTIONS] <FILE|FOLDER>...\n\
         \n\
         Options:\n\
           -a, --analysis NAME   Show one analysis (repeatable; default: all)\n\
           -j, --json PATH       Save the full FlacCompagnon report as JSON\n\
               --no-recursive   Scan only the named folder level\n\
               --no-flac-md5    Read FLAC signatures without verifying them\n\
               --ffmpeg PATH    Use ffmpeg for DSD content analysis\n\
           -h, --help           Show this help\n\
           -V, --version        Show the version\n\
         \n\
         Analyses: {}\n\
         \n\
         Analysis selection filters terminal output. JSON always carries the full\n\
         report so it can be re-imported by the desktop app or Aède.",
        env!("CARGO_PKG_VERSION"),
        ANALYSES.join(", ")
    );
}

fn ffmpeg_on_path() -> Option<String> {
    if let Ok(explicit) = std::env::var("FLACCOMPAGNON_FFMPEG") {
        if Path::new(&explicit).is_file() {
            return Some(explicit);
        }
    }
    let path = std::env::var_os("PATH")?;
    for folder in std::env::split_paths(&path) {
        let candidate = folder.join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

fn display(file: &FileAnalysis, args: &Args) {
    println!("{}", file.path);
    if let Some(error) = &file.error {
        println!("  Error: {error}");
        return;
    }
    if args.selected("authenticity") {
        println!(
            "  Authenticity: {} — {}",
            file.detections.summary, file.detections.detail
        );
    }
    if args.selected("bit-depth") {
        println!(
            "  Bit depth: declared {:?}, effective {:?}",
            file.declared_bits, file.real_bit_depth
        );
    }
    if args.selected("spectrum") {
        println!("  Spectral cutoff: {:?} Hz", file.cutoff_hz);
    }
    if args.selected("stereo") {
        println!(
            "  Stereo: fake {:?}, balance {:?}",
            file.fake_stereo, file.stereo_balance
        );
    }
    if args.selected("phase") {
        println!(
            "  Phase: global {:?}, inverted {:?}, local {:?}",
            file.phase_correlation, file.phase_inverted, file.local_phase
        );
    }
    if args.selected("hf-stereo") {
        println!("  HF stereo: {:?}", file.high_frequency_stereo);
    }
    if args.selected("clipping") {
        println!(
            "  Clipping: {} events, {:.2} dBFS, {:.2} dBTP",
            file.clipping.clip_events, file.clipping.peak_dbfs, file.clipping.true_peak_dbtp
        );
    }
    if args.selected("loudness") {
        println!(
            "  Loudness: integrated {:?} LUFS, peaks {:?}, range {:?} LU",
            file.integrated_lufs, file.loudness_peaks, file.loudness_range_lu
        );
    }
    if args.selected("dynamics") {
        println!("  Dynamic range: {:?} dB", file.dr_db);
    }
    if args.selected("clicks") {
        println!(
            "  Suspected clicks: {:?}",
            file.discontinuities.as_ref().map(|d| d.clicks.count)
        );
    }
    if args.selected("dropouts") {
        println!(
            "  Suspected dropouts: {:?}",
            file.discontinuities.as_ref().map(|d| d.dropouts.count)
        );
    }
    if args.selected("dc-offset") {
        println!("  DC offset: {:?}", file.dc_offset);
    }
    if args.selected("flac-md5") {
        println!("  FLAC MD5: {:?}", file.flac_md5);
    }
    if args.selected("fingerprints") {
        println!(
            "  File MD5: {:?}; CRC32: {:?}",
            file.file_md5, file.file_crc32
        );
    }
}

fn run(args: Args) -> Result<(), String> {
    let paths = core::gather_targets(&args.targets, args.recursive);
    if paths.is_empty() {
        return Err("no supported audio files found".to_string());
    }
    let options = ScanOptions {
        recursive: args.recursive,
        verify_flac_md5: args.verify_flac_md5,
        ffmpeg: args.ffmpeg.clone().or_else(ffmpeg_on_path),
    };
    let files: Vec<_> = paths
        .iter()
        .map(|path| core::analyze_file(path, &options))
        .collect();
    let report = core::folder_report(Path::new(&core::display_root(&args.targets)), files);
    for file in &report.files {
        display(file, &args);
    }
    if let Some(dest) = &args.json {
        core::report::write_json(dest, &report)
            .map_err(|error| format!("{}: {error}", dest.display()))?;
        println!("Saved {}", dest.display());
    }
    if report.files.iter().any(|file| file.error.is_some()) {
        return Err("one or more files could not be analyzed".to_string());
    }
    Ok(())
}

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if matches!(raw.as_slice(), [arg] if arg == "--version" || arg == "-V") {
        println!("flaccompagnon {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    match Args::parse(raw) {
        Ok(None) => help(),
        Ok(Some(args)) => {
            if let Err(error) = run(args) {
                eprintln!("flaccompagnon: {error}");
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("flaccompagnon: {error}\nUse --help for usage.");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/args.rs"]
mod tests;
