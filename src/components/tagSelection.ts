// Pure reductions from "the tags of every file in the selection" down to what
// the panel shows. No React, no DOM — a selection of N files collapses to one
// value per field plus a "they disagree" flag, which is the whole model the
// panel is built on.

import type { CoverArt, FieldEdit, TagSet } from "../types";
import type { TagFieldValue } from "./TagField";
import { TAG_TEXT_FIELDS, type TagTextField } from "./tagLayout";

/// One value per field: the shared value when every file agrees, otherwise
/// empty and flagged `mixed`. Empty-and-mixed matters — it's what lets a field
/// the user never touched stay `Unset` on save instead of overwriting the
/// files that had a different value.
export function fieldValues(tagSets: TagSet[]): Record<TagTextField, TagFieldValue> {
  const out = {} as Record<TagTextField, TagFieldValue>;
  for (const field of TAG_TEXT_FIELDS) {
    const values = new Set(tagSets.map((t) => t[field] ?? ""));
    const mixed = values.size > 1;
    out[field] = { value: mixed ? "" : ([...values][0] ?? ""), mixed };
  }
  return out;
}

/// Tri-state compilation flag: true/false when every file agrees, `null` when
/// they don't (rendered as an indeterminate checkbox).
export function compilationValue(tagSets: TagSet[]): boolean | null {
  const values = new Set(tagSets.map((t) => t.compilation));
  return values.size === 1 ? [...values][0] : null;
}

function sameCover(a: CoverArt, b: CoverArt): boolean {
  return a.mime === b.mime && a.data_base64 === b.data_base64;
}

/// Every distinct cover across the selection, deduped by content rather than
/// by file — so ten tracks of one album yield one cover, not ten.
export function distinctCovers(tagSets: TagSet[], pictureType: string): CoverArt[] {
  const out: CoverArt[] = [];
  for (const t of tagSets) {
    for (const c of t.pictures) {
      if (c.picture_type === pictureType && !out.some((d) => sameCover(d, c))) out.push(c);
    }
  }
  return out;
}

export interface ExtendedRow {
  key: string;
  value: string;
  mixed: boolean;
}

/// Everything lofty read that isn't one of the panel's own fields (ISRC, BPM,
/// ReplayGain, custom frames, ...). A key absent from some files counts as an
/// empty value for those, same convention as the known fields — so "present
/// with the same value everywhere" is the only case not flagged as mixed.
export function extendedRows(tagSets: TagSet[]): ExtendedRow[] {
  const keys = new Set<string>();
  for (const t of tagSets) for (const [k] of t.extra) keys.add(k);
  const rows: ExtendedRow[] = [];
  for (const key of keys) {
    const values = new Set(tagSets.map((t) => t.extra.find(([k]) => k === key)?.[1] ?? ""));
    const mixed = values.size > 1;
    rows.push({ key, value: mixed ? "" : ([...values][0] ?? ""), mixed });
  }
  rows.sort((a, b) => a.key.localeCompare(b.key));
  return rows;
}

/// Overlays a sparse `{ key: FieldEdit }` draft onto the selection's base
/// extended rows — what the extended-tags pop-in shows while edits are
/// staged but not yet pushed up to the panel's own buffer. `Clear` drops the
/// row, `Set` upserts one with `mixed: false` (an edit resolves any
/// disagreement by definition), `Unset` shouldn't appear in a sparse map but
/// is a no-op if it does, same as everywhere else `FieldEdit` is read.
export function applyExtraEdits(rows: ExtendedRow[], edits: Record<string, FieldEdit>): ExtendedRow[] {
  const out = rows
    .filter((r) => edits[r.key] !== "Clear")
    .map((r) => {
      const edit = edits[r.key];
      return edit && edit !== "Unset" && edit !== "Clear"
        ? { key: r.key, value: edit.Set, mixed: false }
        : r;
    });
  for (const [key, edit] of Object.entries(edits)) {
    if (edit === "Unset" || edit === "Clear") continue;
    if (!out.some((r) => r.key === key)) out.push({ key, value: edit.Set, mixed: false });
  }
  out.sort((a, b) => a.key.localeCompare(b.key));
  return out;
}

/// A MusicBrainz Release ID every tagged file agrees on (the normal case for a
/// whole album ripped together), letting the online lookup skip the fuzzy text
/// search and jump straight to that release — the shortcut Picard takes.
export function commonReleaseId(tagSets: TagSet[]): string | null {
  const ids = new Set(
    tagSets.map((t) => t.musicbrainz_release_id).filter((id): id is string => !!id),
  );
  return ids.size === 1 ? [...ids][0] : null;
}
