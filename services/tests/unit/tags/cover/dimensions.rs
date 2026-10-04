use super::*;

// These are dimension-bearing headers built from each format specification;
// raster decoding is deliberately outside this metadata reader's contract.
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend_from_slice(&13u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 2, 0, 0, 0, 0, 0, 0, 0]);
    bytes
}

fn jpeg(width: u16, height: u16) -> Vec<u8> {
    // Independent encoder fixture: FFmpeg's black 2x4 lavfi input, scaled
    // to 2x3 and encoded by mjpeg with yuvj444p. Only the SOF geometry is
    // modified for malformed/oversized metadata cases.
    let mut bytes = include_bytes!("../../../fixtures/cover-2x3.jpg").to_vec();
    let sof = bytes
        .windows(2)
        .position(|marker| marker == [0xff, 0xc0])
        .expect("the encoder fixture has a baseline SOF marker");
    bytes[sof + 5..sof + 7].copy_from_slice(&height.to_be_bytes());
    bytes[sof + 7..sof + 9].copy_from_slice(&width.to_be_bytes());
    bytes
}

fn gif(width: u16, height: u16) -> Vec<u8> {
    let mut bytes = b"GIF89a".to_vec();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, 0, 0x2c, 0, 0, 0, 0]);
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend_from_slice(&[0, 2, 2, 0x44, 0x01, 0, 0x3b]);
    bytes
}

fn bmp(width: i32, height: i32) -> Vec<u8> {
    let mut bytes = vec![0; 54];
    bytes[..2].copy_from_slice(b"BM");
    bytes[14..18].copy_from_slice(&40u32.to_le_bytes());
    bytes[18..22].copy_from_slice(&width.to_le_bytes());
    bytes[22..26].copy_from_slice(&height.to_le_bytes());
    bytes[26..28].copy_from_slice(&1u16.to_le_bytes());
    bytes[28..30].copy_from_slice(&24u16.to_le_bytes());
    bytes
}

fn riff_chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    if !payload.len().is_multiple_of(2) {
        bytes.push(0);
    }
    bytes
}

fn webp(chunks: &[u8]) -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&((chunks.len() + 4) as u32).to_le_bytes());
    bytes.extend_from_slice(b"WEBP");
    bytes.extend_from_slice(chunks);
    bytes
}

fn vp8l(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0x2f];
    bytes.extend_from_slice(&((width - 1) | ((height - 1) << 14)).to_le_bytes());
    riff_chunk(b"VP8L", &bytes)
}

fn vp8(width: u16, height: u16) -> Vec<u8> {
    let mut bytes = vec![0, 0, 0, 0x9d, 0x01, 0x2a];
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    riff_chunk(b"VP8 ", &bytes)
}

fn extended_webp(width: u32, height: u32) -> Vec<u8> {
    let mut header = vec![0; 10];
    header[4..7].copy_from_slice(&(width - 1).to_le_bytes()[..3]);
    header[7..10].copy_from_slice(&(height - 1).to_le_bytes()[..3]);
    let mut chunks = riff_chunk(b"VP8X", &header);
    chunks.extend_from_slice(&vp8l(width.min(16384), height.min(16384)));
    webp(&chunks)
}

fn tiff(width: u32, height: u32, little: bool) -> Vec<u8> {
    let u16_bytes = |value: u16| {
        if little {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        }
    };
    let u32_bytes = |value: u32| {
        if little {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        }
    };
    let mut bytes = if little {
        b"II".to_vec()
    } else {
        b"MM".to_vec()
    };
    bytes.extend_from_slice(&u16_bytes(42));
    bytes.extend_from_slice(&u32_bytes(8));
    bytes.extend_from_slice(&u16_bytes(2));
    for (tag, value) in [(256, width), (257, height)] {
        bytes.extend_from_slice(&u16_bytes(tag));
        bytes.extend_from_slice(&u16_bytes(4));
        bytes.extend_from_slice(&u32_bytes(1));
        bytes.extend_from_slice(&u32_bytes(value));
    }
    bytes.extend_from_slice(&u32_bytes(0));
    bytes
}

#[test]
fn dimensions_are_read_for_every_advertised_cover_format() {
    for (mime, bytes) in [
        ("image/png", png(2, 3)),
        ("image/jpeg", jpeg(2, 3)),
        ("image/gif", gif(2, 3)),
        ("image/bmp", bmp(2, -3)),
        ("image/webp", webp(&vp8l(2, 3))),
        ("image/webp", webp(&vp8(2, 3))),
        ("image/webp", extended_webp(2, 3)),
        ("image/tiff", tiff(2, 3, true)),
        ("image/tiff", tiff(2, 3, false)),
    ] {
        let header = inspect(&bytes).unwrap_or_else(|| panic!("rejected {mime}"));
        assert_eq!((header.mime, header.width, header.height), (mime, 2, 3));
    }
}

#[test]
fn decompression_bombs_are_rejected_in_every_supported_format() {
    for bytes in [
        png(65535, 65535),
        jpeg(65535, 65535),
        gif(65535, 65535),
        bmp(65535, 65535),
        webp(&vp8l(16384, 16384)),
        webp(&vp8(16383, 16383)),
        extended_webp(65535, 65535),
        tiff(65535, 65535, true),
        tiff(65535, 65535, false),
    ] {
        assert!(inspect(&bytes).is_none());
    }
}

#[test]
fn the_announced_64_megapixel_boundary_is_exact() {
    assert_eq!(bounded_dimensions(8000, 8000), Some((8000, 8000)));
    assert_eq!(bounded_dimensions(8001, 8000), None);
}

#[test]
fn malformed_and_unknown_dimensions_are_never_treated_as_zero_sized_images() {
    for bytes in [
        png(0, 3),
        jpeg(0, 3),
        gif(0, 3),
        bmp(0, 3),
        bmp(2, i32::MIN),
        webp(&vp8l(1, 1)[..12]),
        webp(&vp8(0, 3)),
        tiff(0, 3, true),
        webp(&riff_chunk(b"VP8X", &[0; 10])),
        b"GIF89a".to_vec(),
        b"BM".to_vec(),
        b"II\x2a\0".to_vec(),
        b"\xff\xd8\xff".to_vec(),
    ] {
        assert!(inspect(&bytes).is_none());
    }
    for bytes in [gif(2, 3), bmp(2, 3), webp(&vp8l(2, 3)), tiff(2, 3, true)] {
        for end in 0..bytes.len() {
            assert!(
                inspect(&bytes[..end]).is_none(),
                "truncated header at {end}"
            );
        }
    }
}

#[test]
fn a_gif_cannot_hide_a_large_frame_behind_a_small_screen() {
    let mut bytes = gif(2, 3);
    bytes[18..20].copy_from_slice(&65535u16.to_le_bytes());
    bytes[20..22].copy_from_slice(&65535u16.to_le_bytes());
    assert!(inspect(&bytes).is_none());
}

#[test]
fn bmp_cannot_hide_an_independent_encoded_raster_behind_small_dib_dimensions() {
    for (compression, raster) in [(4u32, jpeg(65535, 65535)), (5, png(65535, 65535))] {
        let mut bytes = bmp(2, 3);
        bytes[30..34].copy_from_slice(&compression.to_le_bytes());
        bytes.extend_from_slice(&raster);
        assert!(inspect(&bytes).is_none());
    }
}

#[test]
fn corrupt_webp_chunk_sizes_are_rejected_without_panicking() {
    let mut bytes = webp(&vp8l(2, 3));
    bytes[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(inspect(&bytes).is_none());
    let mut bytes = webp(&vp8l(2, 3));
    bytes[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(inspect(&bytes).is_none());
}

#[test]
fn animated_webp_frames_must_fit_the_canvas_and_match_their_bitstream() {
    let mut header = vec![0; 10];
    header[0] = 2;
    header[4] = 1;
    header[7] = 2;
    let mut frame = vec![0; 16];
    frame[6] = 1;
    frame[9] = 2;
    frame.extend_from_slice(&vp8l(2, 3));
    let mut chunks = riff_chunk(b"VP8X", &header);
    chunks.extend_from_slice(&riff_chunk(b"ANIM", &[0; 6]));
    chunks.extend_from_slice(&riff_chunk(b"ANMF", &frame));
    let image = inspect(&webp(&chunks)).expect("bounded animation header");
    assert_eq!((image.width, image.height), (2, 3));
    frame[6] = 0xff;
    let mut chunks = riff_chunk(b"VP8X", &header);
    chunks.extend_from_slice(&riff_chunk(b"ANMF", &frame));
    assert!(inspect(&webp(&chunks)).is_none());
    frame[6] = 0;
    let mut chunks = riff_chunk(b"VP8X", &header);
    chunks.extend_from_slice(&riff_chunk(b"ANMF", &frame));
    assert!(inspect(&webp(&chunks)).is_none());
}

#[test]
fn cyclic_and_out_of_bounds_tiff_directories_are_rejected_without_panicking() {
    let mut bytes = tiff(2, 3, true);
    bytes[34..38].copy_from_slice(&8u32.to_le_bytes());
    assert!(inspect(&bytes).is_none());
    let mut bytes = tiff(2, 3, true);
    bytes[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(inspect(&bytes).is_none());
}

#[test]
fn a_small_tiff_first_page_cannot_hide_a_large_linked_page() {
    let mut bytes = tiff(2, 3, true);
    bytes[34..38].copy_from_slice(&38u32.to_le_bytes());
    bytes.extend_from_slice(&tiff(2, 3, true)[8..]);
    assert!(inspect(&bytes).is_some());
    bytes[48..52].copy_from_slice(&65535u32.to_le_bytes());
    bytes[60..64].copy_from_slice(&65535u32.to_le_bytes());
    assert!(inspect(&bytes).is_none());
}

#[test]
fn a_small_tiff_first_page_cannot_hide_a_large_sub_ifd() {
    let mut bytes = tiff(2, 3, true);
    let mut child_entry = 330u16.to_le_bytes().to_vec();
    child_entry.extend_from_slice(&4u16.to_le_bytes());
    child_entry.extend_from_slice(&1u32.to_le_bytes());
    child_entry.extend_from_slice(&50u32.to_le_bytes());
    bytes.splice(34..34, child_entry);
    bytes[8..10].copy_from_slice(&3u16.to_le_bytes());
    bytes.extend_from_slice(&tiff(65535, 65535, true)[8..]);
    assert!(inspect(&bytes).is_none());
}

#[test]
fn embedded_and_staged_images_share_the_same_pixel_limit() {
    use super::super::{apply_edits, cover_from_bytes, extract_all, CoverEdit};
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use lofty::{
        picture::{MimeType, Picture},
        tag::{Tag, TagType},
    };
    for bytes in [
        gif(65535, 65535),
        bmp(65535, 65535),
        extended_webp(65535, 65535),
        tiff(65535, 65535, true),
    ] {
        assert!(cover_from_bytes(bytes.clone(), "local or lookup").is_err());
        let mut tag = Tag::new(TagType::VorbisComments);
        // The claimed MIME must not override the bytes' real format.
        tag.push_picture(
            Picture::unchecked(bytes.clone())
                .mime_type(MimeType::Png)
                .build(),
        );
        assert!(extract_all(&tag, "embedded").is_err());
        let edit = CoverEdit::Set {
            mime: "image/png".into(),
            data_base64: STANDARD.encode(&bytes),
            picture_type: "CoverFront".into(),
        };
        assert!(apply_edits(&mut tag, &[edit], "edit").is_err());
    }
}
