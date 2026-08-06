//! FlacCompagnon's Tauri backend — wiring only.
//!
//! This file declares the modules, builds the app and lists the commands the
//! frontend may invoke. It deliberately holds no command bodies and no
//! application logic (see CLAUDE.md): those live in [`commands`], one file per
//! domain, and the real work lives further down still, in `flaccompagnon_core`
//! where it can be tested without a GUI.
//!
//! What this app does to the user's files, in one place:
//!
//! * **Analysis reads only.** Audio files are opened read-only and never
//!   modified by a scan.
//! * **Two things write to audio files**, both only on an explicit Save from
//!   the tag panel: [`commands::tags::write_tags_batch`], and indirectly an
//!   online lookup result the user chose to apply.
//! * **Everything else is written beside the tracks, never into them**: the
//!   CSV/JSON reports, the M3U playlist, the spectrogram PNGs, an extracted
//!   cover. The JSON report is re-importable by dropping it back onto the
//!   window.
//! * **[`lookup`] is the only module that touches the network**, and only when
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
            commands::spectrograms::generate_spectrograms,
            commands::report::save_report_csv,
            commands::report::save_report_json,
            commands::report::save_playlist,
            commands::report::load_report,
            commands::files::reveal_in_folder,
            commands::files::open_folder,
            commands::tags::read_tags_batch,
            commands::tags::write_tags_batch,
            commands::tags::read_cover_image,
            commands::tags::extract_cover_art,
            commands::lookup::lookup_musicbrainz,
            commands::lookup::lookup_musicbrainz_detail,
            commands::lookup::lookup_discogs,
            commands::lookup::lookup_discogs_detail,
            commands::player::play_track,
            commands::player::stop_playback,
        ])
        .run(tauri::generate_context!())
        .expect("error while running FlacCompagnon");
}
