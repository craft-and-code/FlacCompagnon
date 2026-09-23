use super::*;

#[test]
fn maps_known_image_types() {
    assert_eq!(cover_extension("image/jpeg"), "jpg");
    assert_eq!(cover_extension("image/png"), "png");
    assert_eq!(cover_extension("IMAGE/PNG"), "png");
    assert_eq!(cover_extension(" image/webp "), "webp");
}

/// A MIME type from an untrusted file must never be able to steer where
/// the extracted cover is written.
#[test]
fn a_hostile_mime_cannot_escape_the_folder() {
    for mime in [
        "image/../../../etc/passwd",
        "image/png/../../evil",
        "image/sh",
        "",
        "../..",
        "image/png\0.sh",
    ] {
        let ext = cover_extension(mime);
        assert!(
            !ext.contains(['/', '\\', '.', '\0']),
            "{mime} produced {ext}"
        );
    }
}

#[test]
fn first_cover_keeps_the_classic_name() {
    assert_eq!(cover_file_name("image/jpeg", "CoverFront", 1), "cover.jpg");
    // 0 shouldn't occur (indices are 1-based), but degrading to the
    // classic name rather than a nonsensical "cover-0.jpg" is the safer
    // failure mode if it ever does.
    assert_eq!(cover_file_name("image/jpeg", "CoverFront", 0), "cover.jpg");
}

#[test]
fn later_covers_get_a_numbered_name_so_they_do_not_collide() {
    assert_eq!(cover_file_name("image/png", "CoverFront", 2), "cover-2.png");
    assert_eq!(cover_file_name("image/png", "CoverFront", 5), "cover-5.png");
    assert_ne!(
        cover_file_name("image/png", "CoverFront", 1),
        cover_file_name("image/png", "CoverFront", 2)
    );
}

#[test]
fn selected_role_changes_the_export_name() {
    assert_eq!(cover_file_name("image/png", "CoverBack", 1), "back.png");
    assert_eq!(cover_file_name("image/png", "CoverBack", 2), "back-2.png");
    assert_eq!(
        cover_file_name("image/jpeg", "../../../bad", 1),
        "other.jpg"
    );
}
