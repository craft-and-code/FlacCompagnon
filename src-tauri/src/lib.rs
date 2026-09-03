//! FlacCompagnon's Tauri backend — wiring only.
//!
//! This file declares the modules, builds the app and lists the commands the
//! frontend may invoke. It deliberately holds no command bodies and no
//! application logic (see CLAUDE.md): those live in `commands`, one file per
//! domain, and the real work lives further down still, in `flaccompagnon_core`
//! where it can be tested without a GUI.
//!
//! The module and command names below are written as plain code spans, not as
//! rustdoc links. Every module in this crate is private — `run()` is the only
//! public item, since nothing outside links against this lib — so an intra-doc
//! link to any of them would resolve to nothing in generated docs and rustdoc
//! says so (`private_intra_doc_links`). Making the modules `pub` purely to
//! satisfy a link would widen the crate's surface for a documentation tool's
//! benefit, which is the wrong way round. Browse them with
//! `cargo doc --document-private-items` if you want them rendered.
//!
//! What this app does to the user's files, in one place:
//!
//! * **Analysis reads only.** Audio files are opened read-only and never
//!   modified by a scan.
//! * **Two things write to audio files**, both only on an explicit Save from
//!   the tag panel: `commands::tags::write_tags_batch`, and indirectly an
//!   online lookup result the user chose to apply.
//! * **Everything else is written beside the tracks, never into them**: the
//!   CSV/JSON reports, the M3U playlist, the spectrogram PNGs, an extracted
//!   cover. The JSON report is re-importable by dropping it back onto the
//!   window.
//! * **`commands::convert::convert_files` writes new audio files** — the
//!   conversion panel's whole purpose — under a folder the user picks
//!   explicitly; the sources it reads from are never touched.
//! * **One command renames a file without touching its content**:
//!   `commands::rename::rename_file`, from the results table's "click twice
//!   on the name" — the audio and its tags are untouched, only the file's own
//!   name on disk changes, and only its stem: the extension is fixed
//!   server-side too, not just hidden in the UI.
//! * **`lookup` is the only module that touches the network**, and only when
//!   the user clicks "Search online".

mod commands;
mod lookup;
mod menu;
mod playback;
mod spectrogram;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Playback owns a dedicated audio thread for the app's lifetime;
            // it has to be started before any `play_track` can arrive.
            playback::init(app.handle().clone());
            menu::build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::analysis::analyze_paths,
            commands::analysis::ffmpeg_available,
            commands::batch::cancel_task,
            commands::convert::convert_files,
            commands::convert::list_convert_sources,
            commands::spectrograms::generate_spectrograms,
            commands::report::save_report_csv,
            commands::report::save_report_json,
            commands::report::save_playlist,
            commands::report::load_report,
            commands::files::missing_paths,
            commands::files::reveal_in_folder,
            commands::files::open_folder,
            commands::rename::rename_file,
            commands::tags::read_tags_batch,
            commands::tags::write_tags_batch,
            commands::tags::read_cover_image,
            commands::tags::extract_cover_art,
            commands::tags::list_addable_tags,
            commands::lookup::lookup_musicbrainz,
            commands::lookup::lookup_musicbrainz_detail,
            commands::lookup::lookup_discogs,
            commands::lookup::lookup_discogs_detail,
            commands::player::play_track,
            commands::player::stop_playback,
            commands::player::pause_playback,
            commands::player::resume_playback,
            commands::player::seek_playback,
            commands::player::set_volume,
            commands::player::set_muted,
        ])
        .run(tauri::generate_context!())
        .expect("error while running FlacCompagnon");
}
