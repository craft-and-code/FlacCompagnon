//! Standalone entry point for the same analysis and report schema as the app.

use std::path::Path;

mod args;
mod display;
mod integrity;
mod progress;
mod reports;

use args::{help, Args};
use flaccompagnon_core::{self as core, ScanOptions};

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

fn run(args: Args) -> Result<(), String> {
    let paths = progress::with_loading("Finding audio files…", || {
        core::gather_targets(&args.targets, args.recursive)
    });
    if paths.is_empty() {
        return Err("no supported audio files found".into());
    }
    let options = ScanOptions {
        recursive: args.recursive,
        verify_flac_md5: args.verify_flac_md5,
        ffmpeg: args.ffmpeg.clone().or_else(ffmpeg_on_path),
    };
    let groups = reports::groups(&paths, &args);
    let destinations: Vec<_> = groups
        .iter()
        .filter_map(|(_, _, dest)| dest.as_deref())
        .collect();
    for destination in &destinations {
        core::report::validate_destination(destination)
            .map_err(|error| format!("{}: {error}", destination.display()))?;
    }
    let cached = progress::with_loading("Reading saved results…", || {
        reports::cached(&paths, &destinations, &args)
    })?;
    if args.md5_only() {
        return integrity::run(&paths, &cached, &args);
    }
    let mut failed = false;
    for (folder, paths, destination) in groups {
        let mut files = Vec::with_capacity(paths.len());
        let mut changed = false;
        for (index, path) in paths.iter().enumerate() {
            let existing = cached
                .get(&reports::path_key(path))
                .filter(|file| reports::matches_requested(file, path, &args));
            let mut file = if let Some(file) = existing {
                eprintln!(
                    "  ▸ [{}/{}] Reusing {}",
                    index + 1,
                    paths.len(),
                    path.display()
                );
                file.clone()
            } else {
                changed = true;
                progress::with_analysis(path, index, paths.len(), || {
                    core::analyze_file_selected(path, &options, args.selection())
                })
            };
            let original_coverage = file.analyses_run.clone();
            file.restrict_to(args.selection());
            changed |= original_coverage != file.analyses_run;
            failed |= file.error.is_some()
                || matches!(file.flac_md5, Some(core::FlacMd5Status::Mismatch));
            if args.show_results || !args.analyses.is_empty() {
                display::display(&file, &args);
            }
            if let Some(error) = &file.error {
                eprintln!("{}: {error}", path.display());
            }
            files.push(file);
        }
        let report = core::folder_report(&folder, files);
        if let Some(dest) = destination {
            if changed || reports::snapshot_changed(&dest, &report)? {
                core::report::write_json(&dest, &report)
                    .map_err(|error| format!("{}: {error}", dest.display()))?;
                eprintln!("Saved {}", dest.display());
            }
        }
    }
    if failed {
        return Err("one or more files could not be analyzed".into());
    }
    Ok(())
}

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if matches!(raw.as_slice(), [arg] if arg == "--version" || arg == "-V" || arg == "-v") {
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
