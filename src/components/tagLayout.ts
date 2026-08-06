// The tag panel's field list and how it's laid out.
//
// This is the single source of truth for which text tags the panel edits.
// It used to exist twice — once as markup in index.html, once as
// `TAG_TEXT_FIELDS` in TypeScript — which meant adding a tag meant editing
// both, and the two silently drifting was a real possibility. The panel now
// renders from this list alone.

export const TAG_TEXT_FIELDS = [
  "title",
  "artist",
  "album",
  "album_artist",
  "composer",
  "year",
  "genre",
  "track",
  "track_total",
  "disc",
  "disc_total",
  "comment",
] as const;

export type TagTextField = (typeof TAG_TEXT_FIELDS)[number];

/// One full-width field, with a "multiple values" badge when the selection
/// disagrees.
interface TextEntry {
  kind: "text";
  field: TagTextField;
  label: string;
  /// Rendered as a `<textarea>` rather than an `<input>`.
  multiline?: boolean;
}

/// Two half-width fields side by side. Too narrow for a badge, so a mixed
/// value shows a compact "≠" placeholder instead.
interface NarrowRow {
  kind: "narrow-row";
  items: { field: TagTextField; label: string; numeric?: boolean }[];
}

/// Two half-width "n / total" pairs side by side (Track 3/12, Disc 1/2).
interface FractionRow {
  kind: "fraction-row";
  items: { label: string; field: TagTextField; totalField: TagTextField }[];
}

export type TagLayoutEntry = TextEntry | NarrowRow | FractionRow;

export const TAG_LAYOUT: TagLayoutEntry[] = [
  { kind: "text", field: "title", label: "Title" },
  { kind: "text", field: "artist", label: "Artist" },
  { kind: "text", field: "album", label: "Album" },
  { kind: "text", field: "album_artist", label: "Album Artist" },
  { kind: "text", field: "composer", label: "Composer" },
  {
    kind: "narrow-row",
    items: [
      { field: "year", label: "Year", numeric: true },
      { field: "genre", label: "Genre" },
    ],
  },
  {
    kind: "fraction-row",
    items: [
      { label: "Track", field: "track", totalField: "track_total" },
      { label: "Disc", field: "disc", totalField: "disc_total" },
    ],
  },
  { kind: "text", field: "comment", label: "Comment", multiline: true },
];
