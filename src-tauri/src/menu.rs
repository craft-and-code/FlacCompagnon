//! The native menu bar.
//!
//! Clicking an item doesn't do the work itself: it emits a `menu://action`
//! event carrying the item's id, which the frontend (already the sole owner of
//! the report, the display order, cached tags, …) routes to the exact same
//! function its equivalent toolbar button calls. So there is one
//! implementation of each action, not one per trigger — and this module holds
//! no application logic at all.
//!
//! Same items on every desktop platform; on Windows/Linux this renders as an
//! in-window menu bar rather than a screen-top one. A few standard OS items
//! (Cut/Copy/Paste/Select All, so text fields behave as expected) are included
//! alongside the app-specific ones; some are no-ops on Linux where the
//! underlying toolkit doesn't support them, which is harmless.

use tauri::menu::{CheckMenuItem, MenuBuilder, SubmenuBuilder};
use tauri::{Emitter, Listener};

#[cfg(target_os = "macos")]
use tauri::menu::AboutMetadata;

/// The ids this app defines and forwards to the frontend. Anything else in
/// the menu (Cut/Copy/Quit/Hide/…) is handled natively by the OS and must
/// never be forwarded — hence an explicit list rather than "everything".
const APP_ACTIONS: &[&str] = &[
    "export_m3u",
    "export_m3u_extended",
    "export_csv",
    "export_json",
    "reset",
    "spectrogram_size_small",
    "spectrogram_size_full",
];

/// Build and install the menu bar, and wire its events.
pub fn build(app: &tauri::App) -> tauri::Result<()> {
    let handle = app.handle();

    let export_menu = SubmenuBuilder::new(handle, "Export")
        .text("export_m3u", "M3U")
        .text("export_m3u_extended", "M3U Extended")
        .separator()
        .text("export_csv", "CSV")
        .text("export_json", "JSON")
        .build()?;

    // Keep the two sizes together so the default is visible and the full-size
    // variant is discoverable without adding a second top-level File action.
    // They are check items rather than plain labels so the active choice is
    // visible in the native menu; the event handler below keeps them mutually
    // exclusive after either one is clicked.
    let spectrograms_small = CheckMenuItem::with_id(
        handle,
        "spectrogram_size_small",
        "Small Size",
        true,
        true,
        None::<&str>,
    )?;
    let spectrograms_full = CheckMenuItem::with_id(
        handle,
        "spectrogram_size_full",
        "Large Size",
        true,
        false,
        None::<&str>,
    )?;
    let spectrograms_menu = SubmenuBuilder::new(handle, "Spectrograms")
        .items(&[&spectrograms_small, &spectrograms_full])
        .build()?;

    // Quit lives in the macOS app menu below on that platform (the standard
    // place for it); everywhere else, without that menu, File is where it
    // has to be for the app to be quittable from the menu bar at all. The
    // `mut` is only exercised on non-macOS targets — `allow` avoids a
    // platform-dependent "does not need to be mutable" warning on macOS.
    #[allow(unused_mut)]
    let mut file_menu = SubmenuBuilder::new(handle, "File")
        .item(&export_menu)
        .separator()
        .text("reset", "Reset")
        .separator()
        .close_window();
    #[cfg(not(target_os = "macos"))]
    {
        file_menu = file_menu.quit();
    }
    let file_menu = file_menu.build()?;

    let edit_menu = SubmenuBuilder::new(handle, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    // Same as above, mirrored: this `mut` is only exercised on macOS.
    #[allow(unused_mut)]
    let mut menu = MenuBuilder::new(handle);

    // The macOS-only application menu (the one with the app's name, first in
    // the bar). Without an explicit submenu here, macOS has nowhere to put
    // Hide/Hide Others/Show All — Cmd+H silently does nothing, because that
    // shortcut has no menu item to be bound to, not because Tauri or macOS
    // block it. `MenuBuilder`/`SubmenuBuilder` don't inject this menu
    // automatically; a fully custom menu has to add it itself.
    #[cfg(target_os = "macos")]
    {
        let app_menu = SubmenuBuilder::new(handle, "FlacCompagnon")
            .about(Some(AboutMetadata::default()))
            .separator()
            .services()
            .separator()
            .hide()
            .hide_others()
            .show_all()
            .separator()
            .quit()
            .build()?;
        menu = menu.item(&app_menu);
    }

    let menu = menu
        .item(&file_menu)
        .item(&spectrograms_menu)
        .item(&edit_menu)
        .build()?;
    app.set_menu(menu)?;

    // The frontend owns persisted preferences. It sends the saved value back
    // after startup so a previous choice is reflected in the native checkmark
    // before the user opens the menu.
    let small_for_sync = spectrograms_small.clone();
    let full_for_sync = spectrograms_full.clone();
    app.listen("spectrogram://size", move |event| match event.payload() {
        "half" => {
            let _ = small_for_sync.set_checked(true);
            let _ = full_for_sync.set_checked(false);
        }
        "full" => {
            let _ = small_for_sync.set_checked(false);
            let _ = full_for_sync.set_checked(true);
        }
        _ => {}
    });

    let app_handle = app.handle().clone();
    app.on_menu_event(move |_app, event| {
        let id = event.id().as_ref();
        if APP_ACTIONS.contains(&id) {
            if id == "spectrogram_size_small" {
                let _ = spectrograms_small.set_checked(true);
                let _ = spectrograms_full.set_checked(false);
            } else if id == "spectrogram_size_full" {
                let _ = spectrograms_small.set_checked(false);
                let _ = spectrograms_full.set_checked(true);
            }
            let _ = app_handle.emit("menu://action", id);
        }
    });

    Ok(())
}
