// The tag panel: everything about the tags of the currently selected rows.
//
// It composes the pieces rather than implementing them — fields from
// `TagFields`, artwork from `CoverArt`, the edit buffer from `useTagEditor`,
// and the three pop-ins it can open. The imperative handle exists because the
// OS file-drop arrives as a Tauri window event, outside React: the app-level
// handler needs to ask "is the pointer over the cover box?" and, if an image
// lands there, hand it in.

import { useImperativeHandle, useMemo, useRef, useState, type Ref } from "react";
import { X } from "lucide-react";

import type { CoverArt as CoverArtData, LookupRelease, TagSet } from "../types";
import * as api from "../api";
import { commonDir } from "../format";
import { ConfirmDialog } from "./ConfirmDialog";
import { CoverArt } from "./CoverArt";
import { CoverModal } from "./CoverModal";
import { ExtendedTagsModal } from "./ExtendedTagsModal";
import { IconButton } from "./IconButton";
import { LookupModal } from "./LookupModal";
import { TagFields } from "./TagFields";
import type { TagTextField } from "./tagLayout";
import { commonReleaseId, distinctCovers, extendedRows } from "./tagSelection";
import { useTagEditor } from "./useTagEditor";

export interface TagPanelHandle {
  /// Hit-test in logical (CSS) pixels — Tauri reports drop positions in
  /// physical pixels, so the caller divides by the device pixel ratio first.
  containsPoint(cssX: number, cssY: number): boolean;
  /// Stage a dropped image as the new cover for every selected file.
  stageCover(cover: CoverArtData): void;
  /// Toggle the cover box's loading spinner while a dropped image is being
  /// read from disk (the read happens in `useNativeDrop`, outside this
  /// component, which is why it needs an imperative way in).
  setCoverLoading(loading: boolean): void;
  /// True while there's an unsaved edit (a field, the cover, or the
  /// compilation flag) for the current selection — the caller uses this to
  /// decide whether switching to a different selection needs confirming
  /// first, since by the time this component re-renders with new paths the
  /// edit buffer has already been wiped.
  isDirty(): boolean;
  /// Silently discards the current edit buffer — used right before honoring
  /// a confirmed "switch selection anyway" so the wipe that would otherwise
  /// happen implicitly (see `useTagEditor`'s selection-key reset) is instead
  /// an explicit, intentional step.
  discardEdits(): void;
}

export interface TagPanelProps {
  selectedPaths: string[];
  /// Tags for the selection, with unreadable/unsupported files already
  /// dropped — an empty array with a non-empty selection means "no taggable
  /// file here".
  tagSets: TagSet[];
  coverDragOver: boolean;
  onClose: () => void;
  onSaved: (paths: string[]) => void;
  onToast: (msg: string, kind?: "info" | "error") => void;
  ref?: Ref<TagPanelHandle>;
}

export function TagPanel({
  selectedPaths,
  tagSets,
  coverDragOver,
  onClose,
  onSaved,
  onToast,
  ref,
}: TagPanelProps) {
  const coverBoxRef = useRef<HTMLDivElement>(null);
  const [lightboxCover, setLightboxCover] = useState<CoverArtData | null>(null);
  const [extendedOpen, setExtendedOpen] = useState(false);
  const [lookupOpen, setLookupOpen] = useState(false);
  const [coverLoading, setCoverLoading] = useState(false);
  const [deleteCoverOpen, setDeleteCoverOpen] = useState(false);

  const editor = useTagEditor({ paths: selectedPaths, tagSets, onSaved, onToast });

  const covers = useMemo(() => distinctCovers(tagSets), [tagSets]);
  const extended = useMemo(() => extendedRows(tagSets), [tagSets]);
  const releaseId = useMemo(() => commonReleaseId(tagSets), [tagSets]);

  // A staged cover (or deletion) replaces whatever the files currently have,
  // so it's what the carousel must show — otherwise dropping an image, or
  // deleting one, would appear to do nothing until after Save.
  const shownCovers = useMemo(() => {
    const staged = editor.coverEdit;
    if (staged === "Unset") return covers;
    if (staged === "Clear") return [];
    return [
      {
        mime: staged.Set.mime,
        data_base64: staged.Set.data_base64,
        picture_type: staged.Set.picture_type,
        // The staged image's real dimensions aren't known until it round-trips
        // through the backend; the banner shows the byte size it does know.
        width: 0,
        height: 0,
        size_bytes: Math.round((staged.Set.data_base64.length * 3) / 4),
      } satisfies CoverArtData,
    ];
  }, [covers, editor.coverEdit]);

  useImperativeHandle(
    ref,
    () => ({
      containsPoint(cssX: number, cssY: number) {
        const el = coverBoxRef.current;
        if (!el || tagSets.length === 0) return false;
        const r = el.getBoundingClientRect();
        return cssX >= r.left && cssX <= r.right && cssY >= r.top && cssY <= r.bottom;
      },
      stageCover(cover: CoverArtData) {
        editor.stageCover(cover);
      },
      setCoverLoading(loading: boolean) {
        setCoverLoading(loading);
      },
      isDirty() {
        return editor.dirty;
      },
      discardEdits() {
        editor.reset();
      },
    }),
    [tagSets.length, editor],
  );

  // Writes the currently shown cover as a plain file next to the audio —
  // "cover.<ext>" in the selection's common folder, overwriting any previous
  // extract there (same convention as Mp3tag/foobar2000's "export cover").
  // This is a read-only export, so it runs regardless of the `multiple`
  // restriction that applies to relabeling.
  const extractCover = async (cover: CoverArtData) => {
    const dir = commonDir(selectedPaths);
    if (!dir) return;
    try {
      const path = await api.extractCoverArt(dir, cover.mime, cover.data_base64);
      onToast(`Cover saved to ${path}`);
    } catch (e) {
      onToast(String(e), "error");
    }
  };

  const applyLookup = (release: LookupRelease, trackIndex: number | null) => {
    const patch: Partial<Record<TagTextField, string>> = {
      album: release.title,
      album_artist: release.artist,
      artist: release.artist,
    };
    if (release.year) patch.year = release.year;
    if (release.tracks.length > 0) patch.track_total = String(release.tracks.length);
    if (trackIndex != null) {
      const track = release.tracks[trackIndex];
      if (track) {
        patch.title = track.title;
        patch.track = track.position;
      }
    }
    editor.setFields(patch);
    if (release.cover) editor.stageCover(release.cover);
  };

  const taggable = tagSets.length > 0;

  return (
    <aside className="tag-panel">
      <div className="tag-panel-head">
        <h2>Tags</h2>
        <div className="tag-panel-head-right">
          <span className="muted">
            {selectedPaths.length === 1 ? "1 track" : `${selectedPaths.length} tracks`}
          </span>
          <IconButton
            icon={<X size={14} strokeWidth={1.8} />}
            title="Close (deselect)"
            variant="close"
            onClick={onClose}
          />
        </div>
      </div>

      <div ref={coverBoxRef}>
        <CoverArt
          covers={shownCovers}
          dragOver={coverDragOver}
          loading={coverLoading}
          onOpenLightbox={setLightboxCover}
          onRoleChange={editor.setCoverRole}
          onDelete={() => setDeleteCoverOpen(true)}
          onExtract={extractCover}
        />
      </div>

      <div className="tag-panel-body">
        {taggable ? (
          <TagFields
            values={editor.values}
            onFieldChange={editor.setField}
            compilation={editor.compilation}
            onCompilationChange={editor.setCompilation}
            extendedCount={extended.length}
            onOpenExtended={() => setExtendedOpen(true)}
            onOpenLookup={() => setLookupOpen(true)}
          />
        ) : (
          <p className="muted">Tags aren't available for this selection (unsupported format).</p>
        )}
      </div>

      <div className="tag-panel-actions">
        <button
          className="btn btn-ghost"
          disabled={!editor.dirty || editor.saving}
          onClick={editor.reset}
        >
          Reset
        </button>
        <button className="btn" disabled={!editor.dirty || editor.saving} onClick={editor.save}>
          Save
        </button>
      </div>

      <ConfirmDialog
        open={deleteCoverOpen}
        title="Delete cover"
        message={`Remove the cover art from ${
          selectedPaths.length === 1 ? "this track" : `these ${selectedPaths.length} tracks`
        }? This takes effect once you hit Save.`}
        confirmLabel="Delete"
        danger
        onConfirm={() => {
          editor.clearCover();
          setDeleteCoverOpen(false);
        }}
        onCancel={() => setDeleteCoverOpen(false)}
      />
      <CoverModal cover={lightboxCover} onClose={() => setLightboxCover(null)} />
      <ExtendedTagsModal
        open={extendedOpen}
        rows={extended}
        onClose={() => setExtendedOpen(false)}
      />
      <LookupModal
        open={lookupOpen}
        onClose={() => setLookupOpen(false)}
        selectedPaths={selectedPaths}
        existingReleaseId={releaseId}
        prefill={{ artist: editor.values.artist.value, album: editor.values.album.value }}
        onApply={applyLookup}
        onToast={onToast}
      />
    </aside>
  );
}
