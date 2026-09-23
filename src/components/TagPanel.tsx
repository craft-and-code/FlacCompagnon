// The tag panel: everything about the tags of the currently selected rows.
//
// It composes the pieces rather than implementing them — fields from
// `TagFields`, artwork from `CoverArt`, the edit buffer from `useTagEditor`,
// and the three pop-ins it can open. The imperative handle exists because the
// OS file-drop arrives as a Tauri window event, outside React: the app-level
// handler needs to ask "is the pointer over the cover box?" and, if an image
// lands there, hand it in.

import { useImperativeHandle, useMemo, useState, type Ref } from "react";
import { X } from "lucide-react";

import type { CoverArt as CoverArtData, LookupRelease, TagSet } from "../types";
import * as api from "../api";
import { PICTURE_TYPE_LABELS, commonDir, pictureTypeLabel } from "../format";
import "./TagPanel.css";
import { ConfirmDialog } from "./ConfirmDialog";
import { CoverArt } from "./CoverArt";
import { CoverModal } from "./CoverModal";
import { ExtendedTagsModal } from "./ExtendedTagsModal";
import { IconButton } from "./IconButton";
import { LookupModal } from "./LookupModal";
import { TagFields } from "./TagFields";
import type { TagTextField } from "./tagLayout";
import { applyExtraEdits, commonReleaseId, distinctCovers, extendedRows } from "./tagSelection";
import { useTagEditor } from "./useTagEditor";

export interface TagPanelHandle {
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
  /// Every distinct file format in the selection (e.g. `["FLAC"]`, or
  /// `["FLAC", "MP3"]` for a mixed one) — passed through to the extended-tags
  /// pop-in, since extended tags (and what its "+" picker can offer) depend
  /// on the format.
  formats: string[];
  coverDragOver: boolean;
  onClose: () => void;
  onSaved: (paths: string[]) => void;
  onToast: (msg: string, kind?: "info" | "error") => void;
  ref?: Ref<TagPanelHandle>;
}

export function TagPanel({
  selectedPaths,
  tagSets,
  formats,
  coverDragOver,
  onClose,
  onSaved,
  onToast,
  ref,
}: TagPanelProps) {
  // Snapshotted from CoverArt's `covers`/`index` at the moment the image is
  // clicked — not read live from `shownCovers` below, so an edit made while
  // the lightbox is open (unlikely, but the cover box stays interactive
  // behind it) can't shift what the lightbox is showing out from under it.
  const [lightbox, setLightbox] = useState<{ covers: CoverArtData[]; index: number } | null>(
    null,
  );
  const [extendedOpen, setExtendedOpen] = useState(false);
  const [lookupOpen, setLookupOpen] = useState(false);
  const [coverLoading, setCoverLoading] = useState(false);
  const [deleteCoverOpen, setDeleteCoverOpen] = useState(false);
  const [pictureType, setPictureType] = useState("CoverFront");

  const editor = useTagEditor({ paths: selectedPaths, tagSets, onSaved, onToast });

  const covers = useMemo(() => distinctCovers(tagSets, pictureType), [tagSets, pictureType]);
  const extended = useMemo(() => extendedRows(tagSets), [tagSets]);
  // What the "Extended tags (N)" button's count shows — the base rows with
  // whatever's already staged in `editor.extraEdits` overlaid, so an add or
  // remove made in the pop-in is reflected immediately, not just after Save.
  const effectiveExtended = useMemo(
    () => applyExtraEdits(extended, editor.extraEdits),
    [extended, editor.extraEdits],
  );
  const releaseId = useMemo(() => commonReleaseId(tagSets), [tagSets]);

  // The selected type shows its own staged image or deletion immediately;
  // edits to other types stay in the buffer without changing this view.
  const shownCovers = useMemo(() => {
    const staged = editor.pictureEdits[pictureType];
    if (!staged) return covers;
    return staged.preview ? [staged.preview] : [];
  }, [covers, editor.pictureEdits, pictureType]);

  useImperativeHandle(
    ref,
    () => ({
      stageCover(cover: CoverArtData) {
        editor.stageCover(cover, pictureType);
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
    [editor],
  );

  // Export every distinct image of the selected type into the selection's
  // common folder. The backend names front images "cover", back images
  // "back", and numbers additional images of that type.
  const extractCovers = async (toExtract: CoverArtData[]) => {
    const dir = commonDir(selectedPaths);
    if (!dir || toExtract.length === 0) return;
    try {
      const written: string[] = [];
      for (let i = 0; i < toExtract.length; i++) {
        const c = toExtract[i];
        written.push(await api.extractCoverArt(dir, c.mime, c.data_base64, pictureType, i + 1));
      }
      onToast(
        written.length === 1
          ? `Cover saved to ${written[0]}`
          : `${written.length} covers saved to ${dir}`,
      );
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
    if (release.cover) editor.stageCover(release.cover, pictureType);
  };

  const taggable = tagSets.length > 0;

  return (
    <aside className="tag-panel">
      <div className="tag-panel-head">
        <h2>Tags</h2>
        {/* The selected-track count used to live here as text ("N tracks"),
            but ResultsSummary now shows "N selected" above the table, so
            repeating it here was a duplicate — dropped, leaving just the
            close button. */}
        <IconButton
          icon={<X size={14} strokeWidth={1.8} />}
          title="Close (deselect)"
          variant="close"
          className="tag-panel-close"
          onClick={onClose}
        />
      </div>

      <CoverArt
        covers={shownCovers}
        pictureType={pictureType}
        pictureTypes={Object.keys(PICTURE_TYPE_LABELS)}
        onTypeChange={(next) => {
          setPictureType(next);
          setLightbox(null);
        }}
        dragOver={coverDragOver}
        loading={coverLoading}
        onOpenLightbox={(covers, index) => setLightbox({ covers, index })}
        onDelete={() => setDeleteCoverOpen(true)}
        onExtract={extractCovers}
      />

      <div className="tag-panel-body">
        {taggable ? (
          <TagFields
            values={editor.values}
            onFieldChange={editor.setField}
            compilation={editor.compilation}
            onCompilationChange={editor.setCompilation}
            extendedCount={effectiveExtended.length}
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
        title={`Delete ${pictureTypeLabel(pictureType)}`}
        message={`Remove ${pictureTypeLabel(pictureType).toLowerCase()} images from ${
          selectedPaths.length === 1 ? "this track" : `these ${selectedPaths.length} tracks`
        }? This takes effect once you hit Save.`}
        confirmLabel="Delete"
        danger
        onConfirm={() => {
          editor.clearCover(pictureType);
          setDeleteCoverOpen(false);
        }}
        onCancel={() => setDeleteCoverOpen(false)}
      />
      <CoverModal
        covers={lightbox?.covers ?? []}
        index={lightbox?.index ?? 0}
        onNavigate={(index) => setLightbox((l) => (l ? { ...l, index } : l))}
        onClose={() => setLightbox(null)}
      />
      <ExtendedTagsModal
        open={extendedOpen}
        rows={extended}
        pendingEdits={editor.extraEdits}
        selectedPaths={selectedPaths}
        formats={formats}
        onClose={() => setExtendedOpen(false)}
        onSave={editor.mergeExtraEdits}
        onToast={onToast}
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
