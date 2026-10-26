//! Native spectrogram choices share the same exclusive-check and restore logic.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tauri::menu::{CheckMenuItem, Submenu, SubmenuBuilder};
use tauri::{Listener, Wry};

const SIZES: &[(&str, &str, &str)] = &[
    ("spectrogram_size_small", "half", "Standard (1200 × 627)"),
    ("spectrogram_size_full", "full", "Large (1800 × 940)"),
];
const COLORS: &[(&str, &str, &str)] = &[
    ("spectrogram_color_green", "green", "Green"),
    ("spectrogram_color_blue", "blue", "Blue"),
    ("spectrogram_color_amber", "amber", "Amber"),
    ("spectrogram_color_violet", "violet", "Violet"),
];

pub(super) fn is_action(id: &str) -> bool {
    SIZES.iter().chain(COLORS).any(|choice| choice.0 == id)
}

pub(super) fn build(app: &tauri::App) -> tauri::Result<Submenu<Wry>> {
    let size = choices(app, "Size", "spectrogram://size", SIZES)?;
    let color = choices(app, "Color", "spectrogram://color", COLORS)?;
    SubmenuBuilder::new(app.handle(), "Spectrograms")
        .items(&[&size, &color])
        .build()
}

fn choices(
    app: &tauri::App,
    title: &str,
    event: &'static str,
    choices: &'static [(&str, &str, &str)],
) -> tauri::Result<Submenu<Wry>> {
    let mut items = Vec::new();
    let mut menu = SubmenuBuilder::new(app.handle(), title);
    for (index, &(id, _, label)) in choices.iter().enumerate() {
        let item = CheckMenuItem::with_id(app.handle(), id, label, true, index == 0, None::<&str>)?;
        menu = menu.item(&item);
        items.push(item);
    }
    let restored = items.clone();
    let current = Arc::new(AtomicUsize::new(0));
    let restored_current = Arc::clone(&current);
    app.listen(event, move |event| {
        // Tauri serializes emitted strings as JSON; tolerate raw payloads from
        // older webviews while restoring exactly the same native checkmarks.
        let value = serde_json::from_str::<String>(event.payload())
            .unwrap_or_else(|_| event.payload().to_string());
        if let Some(selected) = choices.iter().position(|choice| choice.1 == value) {
            restored_current.store(selected, Ordering::Relaxed);
            for (index, item) in restored.iter().enumerate() {
                let _ = item.set_checked(index == selected);
            }
        }
    });
    app.on_menu_event(move |_, event| {
        if choices.iter().any(|choice| choice.0 == event.id().as_ref()) {
            // The frontend may reject actions while busy. Restore its confirmed
            // choice until its preference event acknowledges an accepted click.
            let selected = current.load(Ordering::Relaxed);
            for (index, item) in items.iter().enumerate() {
                let _ = item.set_checked(index == selected);
            }
        }
    });
    menu.build()
}
