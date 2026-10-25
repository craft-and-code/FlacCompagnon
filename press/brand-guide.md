# FlacCompagnon mark

Use `logos/flaccompagnon-icon.svg` for scalable display or the supplied PNG files for raster delivery. The outlined lowercase f preserves the approved Helvetica Neue Bold shape; no font file is redistributed or required to display it.

| Element          | Value                                       |
| ---------------- | ------------------------------------------- |
| Letter           | `#f1f0eb`                                   |
| Accent point     | `#d8f36a`                                   |
| Surface gradient | `#222a27` → `#101413`                       |
| Canvas           | 1024 × 1024                                 |
| Surface          | x/y 48, width/height 928, corner radius 216 |
| Point            | centre 613/718, radius 44                   |

Preserve the supplied spacing and aspect ratio. Do not stretch the f to match the bounding box of another product's letter. Keep the dot visible and retain the complete icon when placing it beside partner marks. The icon palette does not replace the application's functional analysis/status colours.

## Refresh derived exports

The approved 1024 px PNG is the raster source of truth. After an explicitly approved master update, run:

```sh
node scripts/update-branding.mjs
```

This uses the existing Tauri CLI to regenerate the committed desktop icons and website PNG/ICO assets; it does not build the application or install dependencies. The SVG is copied unchanged to the website. The 512 px press preview remains the separately approved reference. Update sharing artwork separately when its layout or copy changes. Partner sites keep a local copy of the approved raster master.
